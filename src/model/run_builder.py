from __future__ import annotations

from common.models import ModelInput, RunMetadata


def assemble_model_inputs(run: RunMetadata) -> list[ModelInput]:
    """Materialize the operational run from approved inputs and the selected subset.

    TODO:
    - consolidate approved series actually used in execution;
    - point to required rasters, vectors, and auxiliary artifacts;
    - select the operational subset of complete model outputs;
    - validate completeness before run review/publication.
    """
    raise NotImplementedError("Model input assembly is not implemented yet.")
