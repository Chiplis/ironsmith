#!/usr/bin/env python3
"""Append the embedded card catalogue to a packaged engine module.

The engine reserves a slot (`InjectedCatalogSlot` in
crates/ironsmith-wasm/src/embedded_card_catalog.rs) instead of compiling the
catalogue in, so the Rust build never waits for the card bake. This script runs
after wasm-bindgen and wasm-opt: it places the catalogue in a new active data
segment just past the module's initial memory, raises the initial memory to
cover it, and writes the segment's address and length into the slot.

Rust's wasm32 allocator only claims pages it obtains from `memory.grow`, so
the appended range is never handed out as heap.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

MAGIC = b"IRONSMITH-CATLOG"
PLACEHOLDER = b"\xff" * 8
PAGE = 65536

SECTION_MEMORY = 5
SECTION_DATA = 11
SECTION_DATA_COUNT = 12
SECTION_IMPORT = 2


def read_uleb(data: bytes, pos: int) -> tuple[int, int]:
    result = shift = 0
    while True:
        byte = data[pos]
        pos += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if not byte & 0x80:
            return result, pos


def read_sleb(data: bytes, pos: int) -> tuple[int, int]:
    result = shift = 0
    while True:
        byte = data[pos]
        pos += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if not byte & 0x80:
            if byte & 0x40:
                result -= 1 << shift
            return result, pos


def uleb(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def sleb(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        done = (value == 0 and not byte & 0x40) or (value == -1 and byte & 0x40)
        out.append(byte if done else byte | 0x80)
        if done:
            return bytes(out)


def fail(message: str) -> None:
    sys.exit(f"[ERROR] inject-wasm-card-catalog: {message}")


def split_sections(module: bytes) -> list[tuple[int, bytes]]:
    if module[:4] != b"\0asm" or module[4:8] != b"\x01\0\0\0":
        fail("not a WebAssembly 1.0 module")
    sections = []
    pos = 8
    while pos < len(module):
        section_id = module[pos]
        size, pos = read_uleb(module, pos + 1)
        sections.append((section_id, module[pos : pos + size]))
        pos += size
    if pos != len(module):
        fail("truncated module")
    return sections


def check_memory_not_imported(payload: bytes) -> None:
    count, pos = read_uleb(payload, 0)
    for _ in range(count):
        for _ in range(2):
            size, pos = read_uleb(payload, pos)
            pos += size
        kind = payload[pos]
        pos += 1
        if kind == 0x02:
            fail("the module imports its memory; nothing to extend")
        if kind == 0x00:
            _, pos = read_uleb(payload, pos)
        elif kind == 0x01:
            pos += 1
            flags, pos = read_uleb(payload, pos)
            _, pos = read_uleb(payload, pos)
            if flags & 1:
                _, pos = read_uleb(payload, pos)
        elif kind == 0x03:
            pos += 1
            pos += 1
        elif kind == 0x04:
            pos += 1
            _, pos = read_uleb(payload, pos)
        else:
            fail(f"unknown import kind {kind:#x}")


def parse_memory(payload: bytes) -> tuple[int, int | None]:
    count, pos = read_uleb(payload, 0)
    if count != 1:
        fail(f"expected one memory, found {count}")
    flags = payload[pos]
    pos += 1
    if flags not in (0, 1):
        fail(f"unsupported memory limits flags {flags:#x} (shared or 64-bit memory)")
    minimum, pos = read_uleb(payload, pos)
    maximum = None
    if flags & 1:
        maximum, pos = read_uleb(payload, pos)
    return minimum, maximum


def encode_memory(minimum: int, maximum: int | None) -> bytes:
    if maximum is None:
        return uleb(1) + b"\x00" + uleb(minimum)
    return uleb(1) + b"\x01" + uleb(minimum) + uleb(maximum)


def find_slot(payload: bytearray) -> tuple[int, int]:
    """Return (payload offset of the slot fields, linear-memory address)."""
    count, pos = read_uleb(payload, 0)
    found = []
    for _ in range(count):
        flags, pos = read_uleb(payload, pos)
        address = None
        if flags in (0, 2):
            if flags == 2:
                _, pos = read_uleb(payload, pos)
            if payload[pos] != 0x41:
                fail("data segment offset is not an i32.const expression")
            address, pos = read_sleb(payload, pos + 1)
            if payload[pos] != 0x0B:
                fail("data segment offset expression is not a single constant")
            pos += 1
        elif flags != 1:
            fail(f"unknown data segment flags {flags:#x}")
        size, pos = read_uleb(payload, pos)
        body = bytes(payload[pos : pos + size])
        start = body.find(MAGIC)
        while start != -1:
            if address is None:
                fail("the catalogue slot is in a passive data segment")
            found.append((pos + start + len(MAGIC), address + start + len(MAGIC)))
            start = body.find(MAGIC, start + 1)
        pos += size
    if len(found) != 1:
        fail(f"expected one catalogue slot, found {len(found)}")
    return found[0]


def inject(module: bytes, catalog: bytes) -> tuple[bytes, int]:
    sections = split_sections(module)
    ids = [section_id for section_id, _ in sections]
    if SECTION_MEMORY not in ids or SECTION_DATA not in ids:
        fail("the module has no memory or data section")
    for section_id, payload in sections:
        if section_id == SECTION_IMPORT:
            check_memory_not_imported(payload)

    rebuilt = []
    address = None
    for section_id, payload in sections:
        if section_id == SECTION_MEMORY:
            minimum, maximum = parse_memory(payload)
            address = minimum * PAGE
            new_minimum = -(-(address + len(catalog)) // PAGE)
            if maximum is not None and new_minimum > maximum:
                fail(f"catalogue needs {new_minimum} pages but memory allows {maximum}")
            if address + len(catalog) > 0xFFFF_FFFF:
                fail("catalogue does not fit in 32-bit memory")
            payload = encode_memory(new_minimum, maximum)
        elif section_id == SECTION_DATA:
            assert address is not None, "memory section precedes data section"
            data = bytearray(payload)
            field, _ = find_slot(data)
            if bytes(data[field : field + 8]) != PLACEHOLDER:
                fail("the catalogue slot was already filled")
            data[field : field + 4] = address.to_bytes(4, "little")
            data[field + 4 : field + 8] = len(catalog).to_bytes(4, "little")
            count, body_start = read_uleb(data, 0)
            segment = b"\x00\x41" + sleb(address) + b"\x0b" + uleb(len(catalog)) + catalog
            payload = uleb(count + 1) + bytes(data[body_start:]) + segment
        elif section_id == SECTION_DATA_COUNT:
            count, _ = read_uleb(payload, 0)
            payload = uleb(count + 1)
        rebuilt.append((section_id, payload))

    out = bytearray(module[:8])
    for section_id, payload in rebuilt:
        out.append(section_id)
        out += uleb(len(payload))
        out += payload
    return bytes(out), address


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("module", type=Path, help="packaged engine .wasm, rewritten in place")
    parser.add_argument("catalog", type=Path, help="catalogue built by build_card_catalog")
    args = parser.parse_args()
    module = args.module.read_bytes()
    catalog = args.catalog.read_bytes()
    if not catalog:
        fail(f"empty catalogue: {args.catalog}")
    injected, address = inject(module, catalog)
    temporary = args.module.with_name(args.module.name + ".injecting")
    temporary.write_bytes(injected)
    temporary.replace(args.module)
    print(
        f"[INFO] embedded card catalogue ({len(catalog)} bytes) at {address:#x} in {args.module}"
    )


if __name__ == "__main__":
    main()
