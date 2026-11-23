#!/usr/bin/env python3
"""Fail if non-test library code can panic in a way clippy does not catch.

Two checks, both over code with comments, strings and char literals blanked out and every
item marked `#[cfg(test)]` removed:

1. Macro bodies. Clippy does not lint code expanded from a macro, so the crate's
   `deny(clippy::unwrap_used, ...)` never sees the inside of `impl_action_kind!` or the impls
   `hl-rs-derive` generates. Any `macro_rules!`, `quote!` or `quote_spanned!` body (with `{}`,
   `()` or `[]` delimiters) in `src` or `hl-rs-derive/src` fails on a panicking call or macro.

2. Length and condition panics in `src`. `assert!`, `from_slice`, `copy_from_slice` and
   `split_at` panic on a false condition or a wrong length, and no clippy lint denies them.
   Each one needs a `// cannot panic: <why>` comment on its own line or the line above.

`hl-rs-derive` code outside a `quote!` body runs at compile time, where a panic fails the
build rather than a signer, so check 2 does not cover it.
"""

import pathlib
import re
import sys

MACRO_ROOTS = ["src", "hl-rs-derive/src"]
LIBRARY_ROOTS = ["src"]
OPENERS = re.compile(r"\b(?:macro_rules!\s*\w+|quote(?:_spanned)?!)\s*[({\[]")
PANICS = re.compile(
    r"\.unwrap\(\)|\.expect\(|\.unwrap_err\(\)|\.expect_err\("
    r"|\b(?:panic|unreachable|todo|unimplemented)!"
    r"|\b(?:debug_)?assert(?:_eq|_ne)?!"
)
NEEDS_REASON = re.compile(
    r"\b(?:debug_)?assert(?:_eq|_ne)?!|\b(?:copy_)?from_slice\(|\bsplit_at(?:_mut)?\("
)
REASON = "// cannot panic:"
CFG_TEST = re.compile(r"#\[cfg\(test\)\]")
PAIRS = {"{": "}", "(": ")", "[": "]"}
RAW_STRING = re.compile(r'b?r(#*)"')
CHAR_LITERAL = re.compile(r"'(?:\\u\{[0-9a-fA-F]+\}|\\.|[^\\'\n])'")


def sanitize(text: str) -> str:
    """Blank comments, string literals and char literals, keeping every newline."""
    out = list(text)
    i, n = 0, len(text)

    def clear(a: int, b: int) -> None:
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = text[i]
        raw = RAW_STRING.match(text, i)
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j == -1 else j
            clear(i, j)
            i = j
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            clear(i, j)
            i = j
        elif raw and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            close = '"' + raw.group(1)
            j = text.find(close, raw.end())
            j = n if j == -1 else j + len(close)
            clear(i, j)
            i = j
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            clear(i, j + 1)
            i = j + 1
        elif c == "'":
            # A char literal ('x', '\n', '\u{..}') closes within a few characters; a
            # lifetime or loop label ('a, 'outer) does not.
            m = CHAR_LITERAL.match(text, i)
            if m:
                clear(i, m.end())
                i = m.end()
            else:
                i += 1
        else:
            i += 1
    return "".join(out)


def block_end(text: str, start: int) -> int:
    """Index just past the delimiter that closes the one at `start` (text is sanitized)."""
    open_c = text[start]
    close_c = PAIRS[open_c]
    depth = 0
    for i in range(start, len(text)):
        if text[i] == open_c:
            depth += 1
        elif text[i] == close_c:
            depth -= 1
            if depth == 0:
                return i + 1
    return len(text)


def item_end(text: str, start: int) -> int:
    """Index just past the item starting at `start`: its `;`, or the block it opens."""
    depth = 0
    for i in range(start, len(text)):
        c = text[i]
        if c in "([":
            depth += 1
        elif c in ")]":
            depth -= 1
        elif depth == 0 and c == ";":
            return i + 1
        elif depth == 0 and c == "{":
            return block_end(text, i)
    return len(text)


def strip_tests(text: str) -> str:
    """Blank each item marked `#[cfg(test)]` (the attribute and that item only)."""
    while m := CFG_TEST.search(text):
        end = item_end(text, m.end())
        text = text[: m.start()] + re.sub(r"[^\n]", " ", text[m.start() : end]) + text[end:]
    return text


def line_of(text: str, pos: int) -> int:
    return text.count("\n", 0, pos) + 1


def main() -> int:
    found = []
    for root in MACRO_ROOTS:
        for path in sorted(pathlib.Path(root).rglob("*.rs")):
            code = strip_tests(sanitize(path.read_text()))
            for m in OPENERS.finditer(code):
                body_start = m.end() - 1
                body = code[body_start : block_end(code, body_start)]
                for p in PANICS.finditer(body):
                    line = line_of(code, body_start + p.start())
                    found.append(f"{path}:{line}: {p.group(0)} in a macro body")
    for root in LIBRARY_ROOTS:
        for path in sorted(pathlib.Path(root).rglob("*.rs")):
            raw = path.read_text()
            lines = raw.split("\n")
            code = strip_tests(sanitize(raw))
            for p in NEEDS_REASON.finditer(code):
                line = line_of(code, p.start())
                here_and_above = lines[max(line - 2, 0) : line]
                if not any(REASON in seen for seen in here_and_above):
                    found.append(f"{path}:{line}: {p.group(0)} without `{REASON} ...`")
    for f in found:
        print(f)
    if found:
        print("library code can panic here; return an error, or state why it cannot")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
