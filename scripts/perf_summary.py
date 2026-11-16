#!/usr/bin/env python3
"""Prints median values of build/perf/*.json files as a table."""
import json, statistics, sys

files = sys.argv[1:]
runs = [json.load(open(f)) for f in files]
med = lambda xs: statistics.median(xs) if xs else float("nan")
print(f"{len(runs)} runs (median)")
for key in ["launch_to_first_frame_ms", "core_init_us", "dispatch_apply_avg_us", "dispatch_apply_p95_us", "full_snapshot_us", "dispatch_render_avg_ms", "dispatch_render_p95_ms", "total_ms"]:
    vals = [r[key] for r in runs if key in r]
    if vals:
        print(f"{key:28s} {med(vals):10.2f}")
ids = [r["id"] for r in runs[0]["rows"]]
print(f"\n{'test':10s} {'rust_ms':>9s} {'swift_ms':>9s} {'ratio':>7s} {'par_ms':>8s} chk")
for i, tid in enumerate(ids):
    rows = [r["rows"][i] for r in runs]
    g = lambda k: med([x[k] for x in rows if k in x])
    chk = all(x.get("checksum_match", False) for x in rows)
    par = g("rust_par_ms")
    print(f"{tid:10s} {g('rust_ms'):9.2f} {g('swift_ms'):9.2f} {g('ratio'):7.2f} {par:8.2f} {'ok' if chk else 'FAIL'}")
