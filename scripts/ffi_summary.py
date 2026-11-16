#!/usr/bin/env python3
"""Median-of-runs table for test #37 (FFI boundary). Usage: ffi_summary.py ffi-*.json"""
import json
import statistics
import sys

runs = [json.load(open(p)) for p in sys.argv[1:]]
if not runs:
    sys.exit("no files")
keys = ["swift_inline_ns", "rust_per_call_ns", "rust_batched_ns", "overhead_ns"]
print(f"{len(runs)} runs · median ns per operation")
print("| Case | Calls | Swift inline | Rust per call | Rust batched | Overhead / call | Checksums |")
print("|---|---:|---:|---:|---:|---:|:---:|")
for i, row in enumerate(runs[0]["rows"]):
    vals = {}
    for k in keys:
        xs = [r["rows"][i][k] for r in runs if k in r["rows"][i]]
        vals[k] = f"{statistics.median(xs):.2f}" if xs else "—"
    ok = all(r["rows"][i].get("checksum_match") for r in runs)
    print(f"| {row['case']} | {row['calls']:,} | {vals['swift_inline_ns']} | {vals['rust_per_call_ns']} "
          f"| {vals['rust_batched_ns']} | {vals['overhead_ns']} | {'✅' if ok else '❌'} |")
cs = [r["crossing_ns"] for r in runs if "crossing_ns" in r]
if cs:
    print(f"\nScalar crossing estimate (median of runs): {statistics.median(cs):.2f} ns")
