#!/usr/bin/env python3
"""Fail if a `macro_rules!` or `quote!` body in library code can panic.

Clippy does not lint code expanded from a macro, so the crate's `deny(clippy::unwrap_used,
...)` never sees the inside of `impl_action_kind!` or the impls `hl-rs-derive` generates.
This is the backstop for exactly those bodies. Everything from a file's first `#[cfg(test)]`
on is test code and is skipped.
"""

import pathlib
import re
import sys

ROOTS = ["src", "hl-rs-derive/src"]
OPENERS = re.compile(r"\b(?:macro_rules!\s*\w+|quote!)\s*\{")
PANICS = re.compile(
    r"\.unwrap\(\)|\.expect\(|\bpanic!|\bunreachable!|\btodo!|\bunimplemented!"
)


def strip_comments(text: str) -> str:
    return re.sub(r"//[^\n]*", "", text)


def block_end(text: str, start: int) -> int:
    """Index just past the brace that closes the block opened at `start`."""
    depth, i, in_str = 0, start, False
    while i < len(text):
        c = text[i]
        if in_str:
            if c == "\\":
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return len(text)


def main() -> int:
    found = []
    for root in ROOTS:
        for path in sorted(pathlib.Path(root).rglob("*.rs")):
            text = path.read_text()
            cut = text.find("#[cfg(test)]")
            if cut != -1:
                text = text[:cut]
            text = strip_comments(text)
            for m in OPENERS.finditer(text):
                body_start = m.end() - 1
                body = text[body_start : block_end(text, body_start)]
                for p in PANICS.finditer(body):
                    line = text.count("\n", 0, body_start + p.start()) + 1
                    found.append(f"{path}:{line}: {p.group(0)}")
    for f in found:
        print(f)
    if found:
        print("a macro body in library code can panic; return an error instead")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
