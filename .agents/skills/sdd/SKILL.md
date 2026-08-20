---
name: sdd
description: Guide lightweight spec-driven development with one concise, temporary change specification that evolves through discussion and implementation.
---

# Lightweight Spec-Driven Development

Use a single working specification for each non-trivial change. Keep the process useful and proportional to the change rather than imposing ceremony.

## Workflow

1. Inspect the relevant repository code and existing documentation before proposing architecture or implementation.
2. Discuss the change with the user and refine the intended behavior, contracts, constraints, and success criteria.
3. For a non-trivial change, create or update exactly one temporary Markdown file at `docs/changes/<change-name>.md`.
4. Keep that file synchronized as requirements, constraints, discoveries, and implementation decisions change.
5. Use the file as the reference while implementing and verifying the change.
6. After implementation and verification, update the applicable persistent project documentation to describe the current system, then delete the temporary change file.

Keep an unfinished change file in place while work remains. Treat Git history as the archive of completed changes; do not maintain a separate specification archive.

## Change File

Include only sections that add value. A typical file may use:

```markdown
# Change title

## Goal

What should change and why.

## Requirements

- Required behavior
- Important constraints
- Things that must remain unchanged

## Notes

Implementation decisions or discoveries that remain useful during the change.

## Verification

How to know the change works.
```

For simple changes, `Goal` and `Requirements` may be enough. Prefer concise descriptions of behavior, contracts, constraints, and important decisions over low-level implementation steps.

## Guardrails

- Prefer discussion and iteration over generating a complete specification upfront.
- Do not create separate proposal, design, task, or archive files unless the user explicitly requests them.
- Do not automatically generate large task lists.
- Do not turn obvious implementation details into requirements.
- Do not propose architecture before inspecting the existing code.
- Treat `docs/` as documentation of the current system and `docs/changes/` as temporary documentation of work in progress.
- Allow small fixes that do not benefit from a written specification to proceed without a change file, while still inspecting and verifying the change appropriately.
