from __future__ import annotations

from collections.abc import Callable, Iterable


def normalize_provider_codes(
    providers: str | Iterable[str],
    resolver: Callable[[str], object],
) -> tuple[str, ...]:
    values = [providers] if isinstance(providers, str) else list(providers)
    normalized: list[str] = []
    for value in values:
        code = str(value).strip().lower()
        if code and code not in normalized:
            resolver(code)
            normalized.append(code)
    if not normalized:
        raise ValueError("providers must contain at least one provider code.")
    return tuple(normalized)


__all__ = ["normalize_provider_codes"]
