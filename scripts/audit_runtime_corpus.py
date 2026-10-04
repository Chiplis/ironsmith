#!/usr/bin/env python3
"""Run bounded card audits in isolated persistent subprocesses, with resumable evidence.

Build audit_runtime_worker first. Results are keyed by executable + corpus + mode
and recorded transactionally in a separate SQLite database, never engine-status.
An execution observation is not a proof of semantics. Timeouts, crashes, compile
failures, and unexplored branches remain distinct in the report.
"""

import argparse
import concurrent.futures
import fcntl
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import sqlite3
import subprocess
import sys
import threading
import time

from audit_runtime_semantics import scan_text


def checksum(path):
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


class Worker:
    def __init__(self, binary, timeout, log_path):
        self.binary = binary
        self.timeout = timeout
        self.log = open(log_path, "ab", buffering=0)
        self.process = None
        self.buffer = b""
        self.processed = 0

    def stop(self):
        if self.process is not None:
            if self.process.poll() is None:
                self.process.kill()
            self.process.wait()
            self.process.stdin.close()
            self.process.stdout.close()
        self.process = None
        self.buffer = b""
        self.processed = 0

    def close(self):
        self.stop()
        self.log.close()

    def run(self, request):
        if self.process is None:
            self.process = subprocess.Popen(
                [str(self.binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=self.log, bufsize=0,
            )
        started = time.monotonic()
        self.log.write(("\nAUDIT " + request["name"] + "\n").encode())
        try:
            payload = (json.dumps(request, ensure_ascii=False) + "\n").encode()
            # FileIO.write may write fewer bytes than requested on a pipe.
            view = memoryview(payload)
            while view:
                written = self.process.stdin.write(view)
                if not written:
                    raise BrokenPipeError("worker stopped reading")
                view = view[written:]
            while True:
                if b"\n" in self.buffer:
                    line, self.buffer = self.buffer.split(b"\n", 1)
                    try:
                        result = json.loads(line)
                    except json.JSONDecodeError:
                        self.log.write(b"NON_PROTOCOL_STDOUT " + line + b"\n")
                        continue
                    if result.get("name") != request["name"]:
                        raise ValueError("worker returned a mismatched card identity")
                    self.processed += 1
                    # A caught panic can still poison a global cache. Start the
                    # next card fresh; also bound accumulated compiler caches.
                    if (result.get("status") == "panicked"
                            or any(item.get("status") == "panicked" for item in result.get("execution", []))
                            or self.processed >= 128):
                        self.stop()
                    return result
                remaining = self.timeout - (time.monotonic() - started)
                if remaining <= 0:
                    self.stop()
                    return {"name": request["name"], "status": "timeout", "timeout_seconds": self.timeout,
                            "semantic_correctness": "not_proven"}
                ready, _, _ = select.select([self.process.stdout], [], [], remaining)
                if not ready:
                    continue
                chunk = os.read(self.process.stdout.fileno(), 65536)
                if not chunk:
                    code = self.process.wait()
                    self.stop()
                    return {"name": request["name"], "status": "worker_crashed", "exit_code": code}
                self.buffer += chunk
        except (BrokenPipeError, OSError, ValueError) as error:
            self.stop()
            return {"name": request["name"], "status": "worker_protocol_failed", "error": str(error)}


def connect(path):
    db = sqlite3.connect(path)
    db.execute("pragma journal_mode=WAL")
    db.execute("""create table if not exists run (
        run_id text primary key, metadata text not null, created_at text default current_timestamp)""")
    db.execute("""create table if not exists result (
        run_id text not null, card_name text not null, status text not null, result_json text not null,
        recorded_at text default current_timestamp, primary key(run_id,card_name))""")
    return db


def summarize(db, run_id, inventory, out):
    statuses = dict(db.execute("select status,count(*) from result where run_id=? group by status", (run_id,)))
    contracts, executions, semantics = {}, {}, {}
    suspect_cards = 0
    report = open(out / "findings.jsonl", "w")
    for name, raw in db.execute("select card_name,result_json from result where run_id=? order by card_name", (run_id,)):
        row = json.loads(raw)
        suspicious = row["status"] not in ("compiled", "compile_failed")
        for finding in row.get("contracts", []):
            key = finding["severity"] + ":" + finding["code"]
            contracts[key] = contracts.get(key, 0) + 1
            suspicious |= finding["severity"] == "error"
        for observation in row.get("execution", []):
            key = observation["status"]
            executions[key] = executions.get(key, 0) + 1
            suspicious |= key in ("resolution_failed", "direct_resolution_failed", "announcement_failed", "action_or_choice_failed", "panicked", "invariant_failed")
        suspicious |= row.get("has_unimplemented", False) or row.get("parse_lossy", False)
        for candidate in row.get("semantic_candidates", []):
            suspicious = True
            for check in candidate["checks"]:
                key = check["check"]
                semantics[key] = semantics.get(key, 0) + 1
        if suspicious:
            suspect_cards += 1
            report.write(json.dumps(row, ensure_ascii=False) + "\n")
    report.close()
    total = sum(statuses.values())
    result = {
        "run_id": run_id, "canonical_payloads": inventory["payload_count"],
        "recorded": total, "remaining": inventory["payload_count"] - total,
        "excluded_source_names": len(inventory["exclusions"]), "statuses": statuses,
        "explicit_named_face_inventory": inventory.get("explicit_named_face_inventory", False),
        "excluded_face_names": len(inventory.get("face_exclusions", [])),
        "linked_face_transition_coverage": inventory.get("linked_face_transition_coverage", "not_audited"),
        "contract_findings": contracts, "execution_observations": executions,
        "semantic_candidate_checks": semantics,
        "cards_requiring_triage": suspect_cards,
        "all_cards_correct": False,
        "limitation": "No finite fixture sweep proves every game state. Uncovered behavior is not a pass.",
    }
    (out / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/audit_runtime_worker"))
    parser.add_argument("--cards", type=Path, default=Path("cards.json"))
    parser.add_argument("--out", type=Path, default=Path("reports/runtime-audit/corpus"))
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--contracts-only", action="store_true")
    parser.add_argument("--actions-only", action="store_true", help="Probe legal casts/activations, costs, targets and resulting trigger chains instead of injecting events")
    parser.add_argument("--name", action="append", help="Exact name; repeatable, does not redefine full corpus coverage")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--include-definition", action="store_true")
    args = parser.parse_args()
    if args.jobs < 1 or args.timeout <= 0:
        parser.error("jobs and timeout must be positive")
    if args.contracts_only and args.actions_only:
        parser.error("choose contracts-only or actions-only, not both")
    args.out.mkdir(parents=True, exist_ok=True)
    binary = args.binary.resolve()
    metadata = {
        "binary": str(binary), "binary_sha256": checksum(binary),
        "cards": str(args.cards.resolve()), "cards_sha256": checksum(args.cards),
        "contracts_only": args.contracts_only, "timeout_seconds": args.timeout,
        "actions_only": args.actions_only,
        "include_definition": args.include_definition,
        "git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "git_diff_sha256": hashlib.sha256(subprocess.check_output(["git", "diff", "--binary"])).hexdigest(),
        "driver_sha256": checksum(__file__),
        "semantic_scanner_sha256": checksum(Path(__file__).with_name("audit_runtime_semantics.py")),
        "invocation": sys.argv,
    }
    # The binary digest captures untracked compiled source, which git diff omits.
    # Unrelated worktree edits do not invalidate results from identical bytes.
    identity = {key: metadata[key] for key in (
        "binary_sha256", "cards_sha256", "contracts_only", "timeout_seconds",
        "include_definition", "driver_sha256", "semantic_scanner_sha256",
        "actions_only",
    )}
    run_id = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()[:24]
    out = args.out / run_id
    out.mkdir(exist_ok=True)
    run_lock = open(out / "run.lock", "a")
    try:
        fcntl.flock(run_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        parser.error("an audit process already holds this run's lock")
    if not (out / "manifest.json").exists():
        (out / "manifest.json").write_text(json.dumps(metadata, indent=2) + "\n")
    # Retain executable scripts as well as their digests: a later edit must not
    # prevent resuming the exact algorithm that produced these observations.
    for source, name in [(Path(__file__), "audit_runtime_corpus.py"),
                         (Path(__file__).with_name("audit_runtime_semantics.py"), "audit_runtime_semantics.py")]:
        destination = out / name
        if source.resolve() != destination.resolve() and not destination.exists():
            shutil.copy2(source, destination)
    pinned_binary = out / "audit_runtime_worker"
    if not pinned_binary.exists():
        shutil.copy2(binary, pinned_binary)
    if checksum(pinned_binary) != metadata["binary_sha256"]:
        parser.error("worker binary changed while snapshotting; rebuild and retry")
    binary = pinned_binary.resolve()
    inventory_path = out / "inventory.json"
    if not inventory_path.exists():
        temporary = out / "inventory.tmp"
        with open(temporary, "wb") as stream:
            subprocess.run([str(binary), "--inventory", str(args.cards.resolve())], stdout=stream, check=True)
        temporary.replace(inventory_path)
    inventory = json.loads(inventory_path.read_text())
    db = connect(args.out / "results.sqlite3")
    db.execute("insert or ignore into run(run_id,metadata) values (?,?)", (run_id, json.dumps(metadata)))
    db.commit()
    completed = {row[0] for row in db.execute("select card_name from result where run_id=?", (run_id,))}
    cards = [card for card in inventory["cards"] if card["name"] not in completed and (not args.name or card["name"] in args.name)]
    if args.name and set(args.name) - {card["name"] for card in inventory["cards"]}:
        parser.error("requested name absent from canonical inventory")
    if args.limit is not None:
        cards = cards[:args.limit]
    for card in cards:
        card["contracts_only"] = args.contracts_only
        card["include_definition"] = args.include_definition
        card["actions_only"] = args.actions_only
    print(json.dumps({"run_id": run_id, "scheduled": len(cards), "already_recorded": len(completed), "report": str(out)}), flush=True)
    lock = threading.Lock()
    index = 0

    def run_batch(worker_id):
        nonlocal index
        worker = Worker(binary, args.timeout, out / f"worker-{worker_id}.log")
        local_db = connect(args.out / "results.sqlite3")
        try:
            while True:
                with lock:
                    if index >= len(cards):
                        return
                    card = cards[index]
                    index += 1
                row = worker.run(card)
                if row.get("status") == "compiled":
                    row["semantic_candidates"] = scan_text(card["oracle_text"], row.get("compiled_text", ""))
                local_db.execute("insert or replace into result(run_id,card_name,status,result_json) values (?,?,?,?)",
                                 (run_id, card["name"], row["status"], json.dumps(row, ensure_ascii=False)))
                local_db.commit()
        finally:
            worker.close()
            local_db.close()

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        pending = {pool.submit(run_batch, i) for i in range(args.jobs)}
        while pending:
            done, pending = concurrent.futures.wait(pending, timeout=30, return_when=concurrent.futures.FIRST_COMPLETED)
            for future in done:
                future.result()
            counts = dict(db.execute("select status,count(*) from result where run_id=? group by status", (run_id,)))
            print(json.dumps({"recorded": sum(counts.values()), "statuses": counts}), flush=True)
    print(json.dumps(summarize(db, run_id, inventory, out)), flush=True)
    db.close()
    run_lock.close()


if __name__ == "__main__":
    main()
