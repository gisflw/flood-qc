# Standalone IMERG, BNDMET, and ONS acquisition

This increment covers four source routes from the
[acquisition migration](../../docs/migration/acquisition.md). A standalone CLI
and library workflow retains originals and writes normalized files in a new
directory. The provider data is ready for later ingestion by workspace
workflows. Workspace registration, desktop controls, and use in model runs
remain subsequent work.

Accept the increment when offline fixtures verify BNDMET window normalization
and wind pairing, ONS midnight labels and units, and IMERG catalog selection,
grid orientation, accumulation and missing masks. Verify the Rust repository
checks and exercise live routes where provider access is available. Record
provider access separately from scientific or operational qualification.
The [acquisition contract](../../docs/contracts/acquisition.md#standalone-source-acquisition)
defines the supported commands and outputs.
