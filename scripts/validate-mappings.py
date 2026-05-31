#!/usr/bin/env python3
"""mappings/*.yaml の不変条件を検証する（docs/12 §13 / mappings/SCHEMA.md）。

CI・Claude Code の hook・validate-mappings skill から呼ばれる。
違反があれば内容を stderr に出して非ゼロ終了する。

検証する不変条件:
  - 全 yaml を通して entry の `id` が一意
  - `id` / `direction` / `loss` が存在する
  - `direction` ∈ {both, claude_to_codex, codex_to_claude}
  - `loss` ∈ {lossless, lossy, dropped}
  - `degrade` があるなら `loss` == lossy
  - `loss` == dropped のエントリに `transform` を付けない
依存: PyYAML
"""
import sys
import os
import glob

try:
    import yaml
except ImportError:
    print("PyYAML が必要です（pip install pyyaml / uv pip install pyyaml）", file=sys.stderr)
    sys.exit(3)

VALID_DIRECTION = {"both", "claude_to_codex", "codex_to_claude"}
VALID_LOSS = {"lossless", "lossy", "dropped"}


def main() -> int:
    mappings_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "mappings")
    files = sorted(glob.glob(os.path.join(mappings_dir, "*.yaml")))
    if not files:
        print(f"mappings/*.yaml が見つかりません: {mappings_dir}", file=sys.stderr)
        return 2

    seen_ids: dict[str, str] = {}
    errors: list[str] = []
    total = 0

    for path in files:
        name = os.path.basename(path)
        try:
            with open(path, encoding="utf-8") as fp:
                doc = yaml.safe_load(fp)
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{name}: YAML パース失敗: {exc}")
            continue
        if not isinstance(doc, dict):
            errors.append(f"{name}: トップレベルが map ではありません")
            continue
        for entry in doc.get("entries", []) or []:
            total += 1
            eid = entry.get("id")
            if not eid:
                errors.append(f"{name}: id 欠落のエントリがあります: {entry!r:.80}")
                continue
            if eid in seen_ids:
                errors.append(f"id 重複: '{eid}'（{seen_ids[eid]} と {name}）")
            seen_ids[eid] = name
            direction = entry.get("direction")
            loss = entry.get("loss")
            if direction not in VALID_DIRECTION:
                errors.append(f"{eid}: 不正な direction: {direction!r}")
            if loss not in VALID_LOSS:
                errors.append(f"{eid}: 不正な loss: {loss!r}")
            if entry.get("degrade") and loss != "lossy":
                errors.append(f"{eid}: degrade があるが loss != lossy（loss={loss!r}）")
            if loss == "dropped" and entry.get("transform"):
                errors.append(f"{eid}: loss=dropped なのに transform あり: {entry.get('transform')!r}")

    if errors:
        print(f"✗ mappings 検証 NG: {len(errors)} 件（{total} エントリ / {len(files)} ファイル）", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(f"✓ mappings 検証 OK（{total} エントリ / {len(files)} ファイル / id 一意・不変条件充足）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
