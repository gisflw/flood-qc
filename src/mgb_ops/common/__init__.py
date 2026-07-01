"""Shared types, contracts, configuration, and utilities."""

from typing import Any

__all__ = ["dissolve_geometries", "find_upstream_ids"]


def __getattr__(name: str) -> Any:
    if name == "dissolve_geometries":
        from mgb_ops.common.geospatial import dissolve_geometries

        return dissolve_geometries
    if name == "find_upstream_ids":
        from mgb_ops.common.topology import find_upstream_ids

        return find_upstream_ids
    raise AttributeError(name)
