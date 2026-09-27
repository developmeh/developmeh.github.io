#!/usr/bin/env python3
"""Generates corpus/wrapper-soup.html: ordinary content buried in deep
unstyled div/span wrappers (what compression pass 3 collapses), a section
whose every third wrapper carries a border (which pass 3 must keep), and a
wrapped list. The output is committed; rerun only to regenerate it."""

import html
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "wrapper-soup.html"

paras = [
    "Wrapper soup is what a component framework leaves behind: every paragraph arrives inside a dozen divs and spans that carry no style of their own.",
    "Compression pass three removes an element and reparents its children when it has no box properties, no id, no kept attribute, no role and does not change the inherited values of its children.",
    "The loader records how many nodes each pass removed; this page exists to make that number large. Node reduction on the corpus median must reach forty percent by the M2 exit gate.",
    "Inline wrappers count too: a span inside a span inside a span around a single word is one text run after passes three and four.",
    "The second section keeps a border on every third wrapper so the collapse must stop there, and the renderer paints those borders as nested boxes.",
    "Finally, a list where each item is buried in wrappers with class names only, which are dropped by attribute trimming in pass seven.",
]


def wrap(inner, depth):
    for i in range(depth):
        inner = f'<div class="w{i}">{inner}</div>'
    return inner


def spans(text, depth):
    out = []
    for w in text.split(" "):
        s = html.escape(w)
        for i in range(depth):
            s = f'<span class="s{i}">{s}</span>'
        out.append(s)
    return " ".join(out)


def wrap_styled(inner, depth):
    for i in range(depth):
        cls = ' class="keep"' if i % 3 == 2 else f' class="w{i}"'
        inner = f"<div{cls}>{inner}</div>"
    return inner


body = ["<h1>Wrapper soup</h1>"]
for i, p in enumerate(paras[:4]):
    body.append(wrap(f"<p>{spans(p, 3) if i == 3 else html.escape(p)}</p>", 12))
body.append("<h2>Styled every third level</h2>")
body.append(wrap_styled(f"<p>{html.escape(paras[4])}</p>", 9))
body.append("<h2>List in wrappers</h2>")
items = "".join(f"<li>item {k + 1} of the wrapped list</li>" for k in range(5))
body.append(wrap(f"<ul>{items}</ul>", 10))
body.append(wrap(f"<p>{html.escape(paras[5])}</p>", 12))
body.append("<footer>Corpus page 6 of 8 · wrapper soup.</footer>")

doc = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Wrapper soup</title>
<link rel="stylesheet" href="corpus.css">
<style>
  .keep {{ border: 1px solid #1a5fb4; padding: 6px; margin: 4px 0; }}
</style>
</head>
<body>
<div class="page">
{chr(10).join(body)}
</div>
</body>
</html>
"""
OUT.write_text(doc, encoding="utf-8")
print(OUT, len(doc), "bytes", doc.count("<div"), "divs", doc.count("<span"), "spans")
