"""Client-side synchronization for paired Panel DeckGL panes."""
from __future__ import annotations

from typing import Any

import panel as pn
from bokeh.models import CustomJS


_PORTABLE_VIEW_FIELDS = ("longitude", "latitude", "zoom", "pitch", "bearing")


def _sync_callback(target: Any) -> CustomJS:
    """Build the browser callback that moves one rendered DeckGL pane."""
    fields = ", ".join(repr(field) for field in _PORTABLE_VIEW_FIELDS)
    return CustomJS(
        args={"target": target},
        code=f"""
const state = {{}};
for (const field of [{fields}]) {{
  if (this.viewState[field] != null) state[field] = this.viewState[field];
}}
if (!Object.keys(state).length) return;

const lock = window.__mgbDeckGLSyncLock || (window.__mgbDeckGLSyncLock = new Set());
if (lock.has(target.id)) return;

const values = Bokeh.index instanceof Map
  ? Array.from(Bokeh.index.values())
  : Object.values(Bokeh.index);
const children = (view) => {{
  if (!view || !view.child_views) return [];
  return view.child_views instanceof Map
    ? Array.from(view.child_views.values())
    : Array.from(view.child_views);
}};
const findView = (view) => {{
  if (!view) return null;
  if (view.model && view.model.id === target.id) return view;
  for (const child of children(view)) {{
    const found = findView(child);
    if (found) return found;
  }}
  return null;
}};
const targetView = values.map(findView).find(Boolean);
if (!targetView) return;

lock.add(this.id);
try {{
  // Panel's DeckGL view does not consume model.viewState, so move its live
  // MapLibre/DeckGL instances directly as well as retaining Bokeh state.
  target.viewState = state;
  if (targetView._map) {{
    const {{longitude, latitude, ...camera}} = state;
    targetView._map.jumpTo({{center: [longitude, latitude], ...camera}});
  }}
  if (targetView.deckGL) targetView.deckGL.setProps({{viewState: state}});
}} finally {{
  lock.delete(this.id);
}}
""",
    )


def link_deckgl_panes(source: pn.pane.DeckGL, target: pn.pane.DeckGL) -> None:
    """Install bidirectional browser synchronization once both panes render."""
    installed: set[tuple[str, str]] = set()

    def attach() -> None:
        if not source._models or not target._models:
            return
        source_model = next(iter(source._models.values()))[0]
        target_model = next(iter(target._models.values()))[0]
        key = (source_model.id, target_model.id)
        if key in installed:
            return
        source_model.js_on_change("viewState", _sync_callback(target_model))
        target_model.js_on_change("viewState", _sync_callback(source_model))
        installed.add(key)

    # Models exist only after the comparison row is attached to the document.
    document = pn.state.curdoc
    if document is not None:
        document.add_next_tick_callback(attach)
    else:
        attach()


__all__ = ["link_deckgl_panes"]
