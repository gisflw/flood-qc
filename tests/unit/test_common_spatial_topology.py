from __future__ import annotations

import geopandas as gpd
import pandas as pd
import pytest
from shapely.geometry import Polygon

from mgb_ops.common import dissolve_geometries, find_upstream_ids


def test_find_upstream_ids_returns_headwaters_to_target() -> None:
    topology = pd.DataFrame(
        {
            "code": [1, 2, 3, 4, 4],
            "next": [3, 3, 4, 99, 99],
        }
    )

    assert find_upstream_ids(
        topology, 4, id_col="code", id_down_col="next"
    ) == [1, 2, 3, 4]
    assert find_upstream_ids(
        topology, 4, id_col="code", id_down_col="next", include_target=False
    ) == [1, 2, 3]


@pytest.mark.parametrize(
    ("frame", "message"),
    [
        (pd.DataFrame({"id": [1, 1], "down": [2, 3]}), "conflicting"),
        (pd.DataFrame({"id": [1, 2], "down": [2, 1]}), "cycle"),
        (pd.DataFrame({"id": [1, None], "down": [2, 3]}), "integer"),
    ],
)
def test_find_upstream_ids_rejects_invalid_topology(
    frame: pd.DataFrame, message: str
) -> None:
    with pytest.raises(ValueError, match=message):
        find_upstream_ids(frame, 1, id_col="id", id_down_col="down")


def test_find_upstream_ids_rejects_missing_target() -> None:
    with pytest.raises(ValueError, match="Target ID 7"):
        find_upstream_ids(
            pd.DataFrame({"id": [1], "down": [99]}),
            7,
            id_col="id",
            id_down_col="down",
        )


def test_dissolve_geometries_returns_one_feature_and_preserves_crs() -> None:
    frame = gpd.GeoDataFrame(
        {"id": [1, 2]},
        geometry=[
            Polygon([(0, 0), (1, 0), (1, 1), (0, 1)]),
            Polygon([(1, 0), (2, 0), (2, 1), (1, 1)]),
        ],
        crs="EPSG:4326",
    )

    dissolved = dissolve_geometries(frame, attributes={"name": "basin"})

    assert len(dissolved) == 1
    assert dissolved.crs == frame.crs
    assert dissolved.iloc[0]["name"] == "basin"
    assert dissolved.geometry.iloc[0].area == pytest.approx(2)


def test_dissolve_geometries_rejects_empty_input() -> None:
    frame = gpd.GeoDataFrame(geometry=[], crs="EPSG:4326")
    with pytest.raises(ValueError, match="empty"):
        dissolve_geometries(frame)
