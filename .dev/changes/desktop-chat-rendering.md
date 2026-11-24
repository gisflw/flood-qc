# Desktop chat rendering

This increment serves the [desktop and automation migration scope](../migration/desktop_and_automation.md).
Render assistant replies as Markdown as chunks arrive. Keep the bottom composer
and the existing send and newline shortcuts. Reuse the measured space for
off-screen turns so typing and scrolling do not reconstruct the chat history.
Keep the current scroll position when reviewing earlier turns.
Keep the composer height stable while typing. Show a loading indicator while
waiting for the first text, then repaint the reply as new content arrives.

The workspace map and docked pages share a frame with the assistant. Limit the
Data table to visible rows so a large comparison can stay open in another group.
Keep the row order, values, and horizontal scrolling.
Reuse translated map overlays when the camera stays at the same position.
Keep docked page contents inside the group boundary and make wider content
accessible by scrolling. Keep horizontal scrolling for a wide reply inside the
transcript so the composer stays in view.

Check streamed headings, emphasis, code, and tables in a rendered fixture.
Verify that off-screen turns keep the transcript's scroll extent. Use a
synthetic history to compare layout work with and without off-screen culling.
Exercise overflow in split groups, typing in a narrow composer, and the change
from waiting to streaming. Record layout measurements separately from displayed
frame time.
