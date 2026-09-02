#!/usr/bin/env python3
"""Criterion 基准回归检查：比较同一 CI 会话内两次运行的基准，回归 >10% 即失败。

用法（ci.yml bench-regression 任务，同一 job 内两次紧邻运行以消除 runner 噪声）：
    cargo bench -p easydoc --bench read_write -- write_throughput \\
        --save-baseline ref --noplot     # 第一次：参考
    cargo bench -p easydoc --bench read_write -- write_throughput \\
        --save-baseline cur --noplot     # 第二次：本次
    python3 scripts/bench_regression_check.py   # 比较 cur vs ref

设计说明：
- 跨 run 比较不可靠：共享 runner 的 CPU 波动可达 ±30%（实测零代码改动
  也出现 +29% "回归"）。同 job 内两次紧邻运行的负载一致，波动 <5%。
- ref 与 cur 同一次运行生成，无需跨 run 缓存，天然自洽。
"""
from __future__ import annotations

import glob
import json
import os
import sys

REGRESSION_LIMIT = 1.10  # 允许 10% 以内的波动

# 两个 baseline 名：argv 可覆盖（默认 ref=参考、cur=本次）
REF = sys.argv[1] if len(sys.argv) > 1 else "ref"
CUR = sys.argv[2] if len(sys.argv) > 2 else "cur"


def median_estimate(estimates_path: str) -> float:
    with open(estimates_path, encoding="utf-8") as f:
        data = json.load(f)
    return float(data["median"]["point_estimate"])


def main() -> int:
    # 只检查 write_throughput 组（roadmap 性能验收项）
    entries = sorted(glob.glob(f"target/criterion/write_throughput/*/{CUR}/estimates.json"))
    if not entries:
        print(f"未找到本次基准结果（target/criterion/write_throughput/*/{CUR}）")
        return 1

    failures = []
    for cur_path in entries:
        bench_dir = os.path.dirname(os.path.dirname(cur_path))
        bench_id = os.path.basename(bench_dir)
        ref_path = os.path.join(bench_dir, REF, "estimates.json")
        if not os.path.exists(ref_path):
            print(f"[缺参考] {bench_id}: 无 {REF} baseline，跳过")
            continue

        cur = median_estimate(cur_path)
        ref = median_estimate(ref_path)
        ratio = cur / ref
        delta = (ratio - 1.0) * 100.0
        status = "OK" if ratio <= REGRESSION_LIMIT else "REGRESSION"
        print(f"[{status:>10}] {bench_id}: ref={ref:.3e} cur={cur:.3e} Δ={delta:+.2f}%")

        if ratio > REGRESSION_LIMIT:
            failures.append(f"{bench_id}: Δ={delta:+.2f}% 超出 10% 回归阈值")

    if failures:
        print("\n基准回归门禁失败：")
        for f in failures:
            print(f"  - {f}")
        return 1

    print("\n基准回归门禁通过（≤10%）。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
