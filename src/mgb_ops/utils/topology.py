from __future__ import annotations

from collections import defaultdict

import pandas as pd


def _integer_series(frame: pd.DataFrame, column: str) -> pd.Series:
    if column not in frame:
        raise ValueError(f"Topology table is missing required column {column!r}.")
    try:
        values = pd.to_numeric(frame[column], errors="raise")
    except (TypeError, ValueError) as exc:
        raise ValueError(f"Topology column {column!r} must contain integer values.") from exc
    if values.isna().any() or (values % 1 != 0).any():
        raise ValueError(f"Topology column {column!r} must contain integer values.")
    return values.astype("int64")


def find_upstream_ids(
    frame: pd.DataFrame,
    target_id: int,
    *,
    id_col: str,
    id_down_col: str,
    include_target: bool = True,
) -> list[int]:
    """Return a deterministic headwaters-to-outlet list for one contributing basin."""
    ids = _integer_series(frame, id_col)
    downstream_ids = _integer_series(frame, id_down_col)
    edges = pd.DataFrame({"id": ids, "downstream": downstream_ids}).drop_duplicates()

    conflicts = edges.groupby("id", sort=False)["downstream"].nunique()
    ambiguous = sorted(int(value) for value in conflicts[conflicts > 1].index)
    if ambiguous:
        raise ValueError(
            "Topology contains IDs with conflicting downstream mappings: "
            f"{ambiguous}."
        )

    downstream_by_id = {
        int(row.id): int(row.downstream) for row in edges.itertuples(index=False)
    }
    try:
        target_numeric = float(target_id)
    except (TypeError, ValueError) as exc:
        raise ValueError("target_id must be an integer value.") from exc
    if not target_numeric.is_integer():
        raise ValueError("target_id must be an integer value.")
    target = int(target_numeric)
    if target not in downstream_by_id:
        raise ValueError(f"Target ID {target} was not found in topology column {id_col!r}.")

    # Validate the complete functional graph so a malformed source cannot produce
    # plausible-looking but incomplete basin membership.
    complete: set[int] = set()
    for start in sorted(downstream_by_id):
        path: list[int] = []
        positions: dict[int, int] = {}
        current = start
        while current in downstream_by_id and current not in complete:
            if current in positions:
                cycle = path[positions[current] :] + [current]
                raise ValueError(f"Topology contains a cycle: {cycle}.")
            positions[current] = len(path)
            path.append(current)
            current = downstream_by_id[current]
        complete.update(path)

    upstream_by_id: dict[int, list[int]] = defaultdict(list)
    for mini_id, downstream_id in downstream_by_id.items():
        if mini_id != downstream_id:
            upstream_by_id[downstream_id].append(mini_id)
    for values in upstream_by_id.values():
        values.sort()

    ordered: list[int] = []
    stack: list[tuple[int, bool]] = [(target, False)]
    while stack:
        mini_id, expanded = stack.pop()
        if expanded:
            ordered.append(mini_id)
            continue
        stack.append((mini_id, True))
        stack.extend(
            (upstream_id, False)
            for upstream_id in reversed(upstream_by_id.get(mini_id, []))
        )
    return ordered if include_target else ordered[:-1]


__all__ = ["find_upstream_ids"]
