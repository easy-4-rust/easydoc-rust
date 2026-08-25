#!/usr/bin/env python3
"""Criterion 基准回归检查：比较本次运行与上次缓存的基准，回归 >10% 即失败。

用法（CI bench-regression 任务）：
    cargo bench --bench read_write -- write_throughput \\
        --baseline prev --save-baseline cur --noplot
    python3 scripts/bench_regression_check.py

流程：
1. 读取 target/criterion/<group>/<id>/{prev,cur}/estimates.json 的 median；
2. cur 比 prev 慢超过 10% → 打印差异并以非零退出（门禁失败）；
3. 无 prev（首次运行，建立基准）→ 通过；
4. 通过后把 cur 提升为 prev，供下一次 CI 运行比较（同一 ubuntu 硬件类）。
"""
from __future__ import annotations

import glob
import json
import os
import shutil
import sys

REGRESSION_LIMIT = 1.10  # 允许 10% 以内的波动


def median_estimate(estimates_path: str) -> float:
    with open(estimates_path, encoding="utf-8") as f:
        data = json.load(f)
    return float(data["median"]["point_estimate"])


def main() -> int:
    # 只检查 write_throughput 组（roadmap 性能验收项）
    entries = sorted(
        glob.glob("target/criterion/write_throughput/*/cur/estimates.json")
    )
    if not entries:
        print("未找到本次基准结果（target/criterion/write_throughput/*/cur）")
        return 1

    failures = []
    for cur_path in entries:
        bench_dir = os.path.dirname(os.path.dirname(cur_path))
        bench_id = os.path.basename(bench_dir)
        prev_path = os.path.join(bench_dir, "prev", "estimates.json")

        cur = median_estimate(cur_path)
        if not os.path.exists(prev_path):
            print(f"[基准建立] {bench_id}: cur={cur:.3e} ns（无历史基准）")
            # 首次运行：把 cur 提升为 prev，供下次比较
            prev_dir = os.path.join(bench_dir, "prev")
            if os.path.isdir(prev_dir):
                shutil.rmtree(prev_dir)
            shutil.copytree(os.path.join(bench_dir, "cur"), prev_dir)
            continue

        prev = median_estimate(prev_path)
        ratio = cur / prev
        delta = (ratio - 1.0) * 100.0
        status = "OK" if ratio <= REGRESSION_LIMIT else "REGRESSION"
        print(f"[{status:>10}] {bench_id}: prev={prev:.3e} cur={cur:.3e} "
              f"Δ={delta:+.2f}%")

        if ratio > REGRESSION_LIMIT:
            failures.append(f"{bench_id}: Δ={delta:+.2f}% 超出 10% 回归阈值")

        # 通过后把 cur 提升为 prev
        prev_dir = os.path.join(bench_dir, "prev")
        if os.path.isdir(prev_dir):
            shutil.rmtree(prev_dir)
        shutil.copytree(os.path.join(bench_dir, "cur"), prev_dir)

    if failures:
        print("\n基准回归门禁失败：")
        for f in failures:
            print(f"  - {f}")
        return 1

    print("\n基准回归门禁通过（≤10%）。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
