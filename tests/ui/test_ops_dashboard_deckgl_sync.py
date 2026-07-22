from __future__ import annotations

import panel as pn

from apps.ops_dashboard.services.deckgl_sync import link_deckgl_panes


def test_deckgl_sync_bridge_links_both_rendered_panes() -> None:
    pn.extension("deckgl")
    original = pn.pane.DeckGL({"layers": []})
    corrected = pn.pane.DeckGL({"layers": []})
    pn.Row(original, corrected).get_root()

    link_deckgl_panes(original, corrected)

    original_model = next(iter(original._models.values()))[0]
    corrected_model = next(iter(corrected._models.values()))[0]
    original_callbacks = original_model.js_property_callbacks["change:viewState"]
    corrected_callbacks = corrected_model.js_property_callbacks["change:viewState"]

    assert len(original_callbacks) == 1
    assert len(corrected_callbacks) == 1
    code = original_callbacks[0].code
    assert "window.__mgbDeckGLSyncLock" in code
    assert "targetView._map.jumpTo" in code
    assert "targetView.deckGL.setProps" in code
    for field in ("longitude", "latitude", "zoom", "pitch", "bearing"):
        assert repr(field) in code
