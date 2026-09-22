# End-to-End Tests

The Panel dashboard smoke test is opt-in because it launches a browser and a
local Bokeh server. Install the development dependencies and Chromium, then run:

```bash
uv run playwright install chromium
RUN_DASHBOARD_BROWSER_TESTS=1 uv run pytest tests/e2e
```

The test creates a controlled workspace, serves the dashboard, clicks its known
station on the DeckGL map, and checks the corresponding summary and chart.
