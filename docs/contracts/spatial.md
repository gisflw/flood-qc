# Local geometry and topology

The headless store APIs read the regional geometry without a Python runtime. `read_basin_geometry(path)` reads ordered `Mini`, `Xcen` and `Ycen`
columns from MINI.gtp. Forcing-only files need no topology columns.
`read_mini_topology(path)` reads those same centers plus `MiniJus` and local
`Area_(km2)` in one source snapshot. `CatID` is not the mini identifier;
`AreaM_(km2)` is cumulative area and is not used as a weight.

`MiniTopology` contains a positive unique mini ID, positive finite local area in
square kilometres, and an optional downstream mini ID. MINI.gtp uses `-1` for an
outlet. Invalid IDs, unknown downstream references and cycles fail.
`validate_mini_topology` applies these checks to supplied values as well.
`analysis::upstream_minis(topology, outlet)` includes the selected outlet and
returns members in topology input order. It does not infer drainage from polygon
coordinates or calculate area in geographic degrees. Weighted rainfall and
coverage use these local areas through the [series APIs](series.md#time-windows-and-accumulation).

`read_mini_map_layer(path, name, topology)` reads one named vector layer, validates
its mini IDs against the topology, transforms its declared EPSG CRS to WGS84
longitude/latitude, and groups geometry paths by mini. Multiple parts retain one
mini identity and do not create extra area weights. The configured regional layers
are `mini_segments` and `mini_catchments`; a missing or invalid layer returns a
finding to its caller, without invalidating unrelated data or activities.
`read_map_layers` remains available for geographic boundary inspection.
Unsupported geometry/CRS, undersized or degenerate polygon rings, and mismatched
IDs fail. The vector library stores ring closure implicitly; returned polygon
paths explicitly repeat the first point. Segment and catchment layers require
line and polygon geometry respectively. The pinned Terrio vector/CRS integration supplies these readers.

`ObservationStore::station_locations()` returns detached station metadata and an
optional explicitly supplied offset row. That row supplies the station's known
mini link and metre offset. An absent row leaves both unknown; coordinates alone
do not establish a mini association. Missing coordinates remain absent, allowing
selection through station identity without inventing a map position. Supplied
reference thresholds are separate [observation records](observations.md#station-reference-levels),
not model offsets.
