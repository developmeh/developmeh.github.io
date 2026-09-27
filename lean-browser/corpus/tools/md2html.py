#!/usr/bin/env python3
"""Render a developmeh.com post (Zola markdown) into a frozen corpus page.

Zola is not available in the build environment, so this is a small
stdlib-only converter that approximates the site's markup: the theme's
`main > article > #wrap` structure, the byline, the sidebar nav after the
article, Zola-style syntax-highlighted `<pre data-lang>` blocks with inline
`style=""` spans (the `ruby` theme), and heading anchors.

It supports what the two chosen posts use: TOML front matter, ATX
headings, paragraphs, fenced code, unordered/ordered lists, blockquotes,
emphasis (`*`/`_`/`**`/`__`), inline code, links. The author's content is
copied verbatim into the corpus; only presentation is generated.

    md2html.py <post.md> <out.html> [--section "Tech dives"]
"""

import html
import re
import sys
from pathlib import Path

KEYWORDS = {
    "bash": "if then else elif fi for while do done in case esac function return local export "
    "command echo exit set trap source declare readonly shift test true false".split(),
    "sh": "if then else elif fi for while do done in case esac function return local export "
    "command echo exit set trap source declare readonly shift test true false".split(),
    "rust": "fn let mut pub use struct enum impl match if else for while loop return mod crate "
    "self Self const static trait where as in ref move unsafe async await dyn type".split(),
    "go": "func package import var const type struct interface map chan go defer return if else "
    "for range switch case default break continue select nil true false".split(),
}
# Colours from Zola's `ruby` highlight theme (approximate).
C_BG = "#2b2b2b"
C_FG = "#e6e1dc"
C_KW = "#cc7833"
C_STR = "#a5c261"
C_CMT = "#bc9458"
C_VAR = "#d0d0ff"


def highlight(code, lang):
    """Very small tokenizer: comments, strings, keywords, `$vars`."""
    kws = set(KEYWORDS.get(lang, []))
    out = []
    i = 0
    n = len(code)
    while i < n:
        c = code[i]
        if c == "#" and lang in ("bash", "sh", "") and (i == 0 or code[i - 1] in "\n \t"):
            j = code.find("\n", i)
            j = n if j < 0 else j
            out.append(f'<span style="color:{C_CMT};">{html.escape(code[i:j])}</span>')
            i = j
        elif c == "/" and lang in ("rust", "go") and code.startswith("//", i):
            j = code.find("\n", i)
            j = n if j < 0 else j
            out.append(f'<span style="color:{C_CMT};">{html.escape(code[i:j])}</span>')
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and code[j] != c:
                j += 2 if code[j] == "\\" else 1
            j = min(j + 1, n)
            out.append(f'<span style="color:{C_STR};">{html.escape(code[i:j])}</span>')
            i = j
        elif c == "$" and lang in ("bash", "sh"):
            m = re.match(r"\$(\{[^}]*\}|[A-Za-z_@#?0-9]+)", code[i:])
            j = i + (m.end() if m else 1)
            out.append(f'<span style="color:{C_VAR};">{html.escape(code[i:j])}</span>')
            i = j
        elif c.isalpha() or c == "_":
            m = re.match(r"[A-Za-z_][A-Za-z0-9_\-]*", code[i:])
            word = m.group(0)
            j = i + m.end()
            if word in kws:
                out.append(f'<span style="color:{C_KW};">{word}</span>')
            else:
                out.append(html.escape(word))
            i = j
        else:
            out.append(html.escape(c))
            i += 1
    return "".join(out)


INLINE_CODE = re.compile(r"`([^`]+)`")
LINK = re.compile(r"\[([^\]]+)\]\(([^)\s]+)\)")
STRONG = re.compile(r"(\*\*|__)(.+?)\1")
EM = re.compile(r"(?<![*\w])(\*|_)(?!\s)(.+?)(?<!\s)\1(?![*\w])")


def inline(text):
    """Code spans are replaced by placeholders first so `**Stub `x`**`
    still pairs its emphasis markers across the code span."""
    codes = []

    def stash(m):
        codes.append(f"<code>{html.escape(m.group(1))}</code>")
        return f"\x00{len(codes) - 1}\x00"

    text = INLINE_CODE.sub(stash, text)
    text = _inline_no_code(text)
    return re.sub(r"\x00(\d+)\x00", lambda m: codes[int(m.group(1))], text)


def _inline_no_code(text):
    text = html.escape(text, quote=False)
    text = LINK.sub(lambda m: f'<a href="{m.group(2)}">{m.group(1)}</a>', text)
    text = STRONG.sub(lambda m: f"<strong>{m.group(2)}</strong>", text)
    text = EM.sub(lambda m: f"<em>{m.group(2)}</em>", text)
    return text


def slugify(s):
    s = re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")
    return s or "section"


def front_matter(text):
    if not text.startswith("+++"):
        return {}, text
    end = text.index("\n+++", 3)
    fm = text[3:end]
    body = text[end + 4:]
    meta = {}
    for line in fm.splitlines():
        m = re.match(r'^(\w+)\s*=\s*"?([^"\n]*)"?\s*$', line)
        if m and m.group(1) not in meta:
            meta[m.group(1)] = m.group(2)
    m = re.search(r"topics\s*=\s*\[(.*?)\]", fm)
    meta["topics"] = re.findall(r'"([^"]+)"', m.group(1)) if m else []
    return meta, body


def render_body(md):
    lines = md.splitlines()
    out = []
    i = 0
    para = []

    def flush():
        if para:
            out.append(f"<p>{inline(' '.join(para))}</p>")
            para.clear()

    while i < len(lines):
        line = lines[i]
        if line.startswith("```"):
            flush()
            lang = line[3:].strip().split(",")[0]
            j = i + 1
            buf = []
            while j < len(lines) and not lines[j].startswith("```"):
                buf.append(lines[j])
                j += 1
            code = "\n".join(buf) + "\n"
            attrs = f' data-lang="{lang}"' if lang else ""
            cls = f' class="language-{lang}"' if lang else ""
            out.append(
                f'<pre{attrs} style="background-color:{C_BG};color:{C_FG};"{cls}>'
                f"<code{cls}{attrs}>{highlight(code, lang)}</code></pre>"
            )
            i = j + 1
            continue
        m = re.match(r"^(#{1,6})\s+(.*)$", line)
        if m:
            flush()
            level = len(m.group(1))
            title = m.group(2).strip()
            sid = slugify(title)
            out.append(
                f'<h{level} id="{sid}">{inline(title)}'
                f'<a class="anchor" href="#{sid}" aria-label="Anchor link for: {sid}">🔗</a></h{level}>'
            )
            i += 1
            continue
        if re.match(r"^\s*[-*]\s+", line) or re.match(r"^\s*\d+\.\s+", line):
            flush()
            ordered = bool(re.match(r"^\s*\d+\.", line))
            tag = "ol" if ordered else "ul"
            items = []
            while i < len(lines) and (
                re.match(r"^\s*[-*]\s+", lines[i]) or re.match(r"^\s*\d+\.\s+", lines[i])
                or (lines[i].startswith("  ") and items)
            ):
                l = lines[i]
                if re.match(r"^\s*([-*]|\d+\.)\s+", l):
                    items.append(re.sub(r"^\s*([-*]|\d+\.)\s+", "", l))
                else:
                    items[-1] += " " + l.strip()
                i += 1
            out.append(f"<{tag}>" + "".join(f"<li>{inline(it)}</li>" for it in items) + f"</{tag}>")
            continue
        if line.startswith(">"):
            flush()
            buf = []
            while i < len(lines) and lines[i].startswith(">"):
                buf.append(lines[i][1:].strip())
                i += 1
            out.append(f"<blockquote><p>{inline(' '.join(buf))}</p></blockquote>")
            continue
        if line.strip() == "":
            flush()
            i += 1
            continue
        if line.strip() in ("---", "***"):
            flush()
            out.append("<hr>")
            i += 1
            continue
        para.append(line.strip())
        i += 1
    flush()
    return "\n".join(out)


NAV = [
    ("Devex", "/devex/", ["CI/CD", "Copying life", "The perfect dev env"]),
    ("I made a thing", "/i-made-a-thing/", ["Catalyst orchestrator", "The magic of stubbing sh"]),
    ("Projects", "/projects/", ["Beamlet", "Kwike", "Wavelet"]),
    ("Soft-wares", "/soft-wares/", ["The good sergeant", "Sufficient complexity"]),
    ("Tech dives", "/tech-dives/", ["Testing shell scripts", "Judgement-capable circuits"]),
]


def page(meta, body_html, section):
    title = meta.get("title", "Untitled")
    date = meta.get("date", "")
    updated = meta.get("updated", "")
    topics = "".join(
        f'<a class="topic-chip" href="/topics/{slugify(t)}/">{html.escape(t)}</a>'
        for t in meta.get("topics", [])
    )
    words = len(re.findall(r"\w+", body_html))
    updated_html = (
        f'<span class="byline-sep">·</span><span class="byline-updated">updated '
        f'<time datetime="{updated}">{updated}</time></span>'
        if updated and updated != date
        else ""
    )
    nav = []
    for name, href, pages in NAV:
        checked = " checked" if name == section else ""
        items = ""
        for p in pages:
            active = ' class="active"' if p == title else ""
            items += f'<li{active}><a href="{href}{slugify(p)}/">{html.escape(p)}</a></li>'

        nav.append(
            f'<input class="tree-toggle" type="checkbox" id="{slugify(name)}"{checked}>'
            f'<label class="tree-toggle-label" for="{slugify(name)}">{name}</label>'
            f'<ul class="subtree">{items}</ul>'
        )
    has_h1 = "<h1" in body_html
    h1 = "" if has_h1 else f"<h1>{html.escape(title)}</h1>"
    return f"""<!DOCTYPE HTML>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <link rel="stylesheet" href="blog/main.css">
    <link rel="stylesheet" href="blog/custom.css">
    <meta name="description" content="{html.escape(meta.get('desc', ''), quote=True)}">
    <meta name="author" content="Paul Scarrone">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{html.escape(title)} | developmeh</title>
    <link rel="alternate" type="application/rss+xml" title="RSS" href="/rss.xml">
    <script>/* analytics stub: scripts are never run in document mode */</script>
</head>
<body>
<main>
    <article>
        <div id="wrap">
<div class="byline">
  <span class="byline-author">by <a href="/landings/about/" rel="author">Paul Scarrone</a></span>
  <span class="byline-sep">·</span>
  <time datetime="{date}">{date}</time>{updated_html}
  <span class="byline-sep">·</span>
  <span class="byline-reading">{max(1, words // 200)} min read</span>
  <span class="byline-topics">{topics}</span>
</div>
{h1}
{body_html}
        <div class="discussion-footer">
            <h3>Discussion</h3>
            <p class="discussion-cta">
                <a href="{html.escape(meta.get('discussion_url', 'https://github.com/orgs/developmeh/discussions'))}" target="_blank" rel="noopener" class="github-discussion-link">
                    <svg class="github-logo" height="16" width="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
                    Discuss on GitHub
                </a>
            </p>
        </div>
        </div>
    </article>
    <nav aria-label="Site navigation">
        <div class="nav-header">
            <a class="brand" href="/">
                <span class="brand-name">developmeh</span>
                <span class="brand-tag">Develop &macr;\\_(&#12484;)_/&macr;</span>
            </a>
            <a href="#" id="mobile" class="ms-Icon--GlobalNavButton" role="button" aria-label="Toggle navigation menu"></a>
        </div>
        <div id="trees">
{"".join(nav)}
        </div>
    </nav>
</main>
</body>
</html>
"""


def main():
    args = sys.argv[1:]
    section = "Tech dives"
    if "--section" in args:
        k = args.index("--section")
        section = args[k + 1]
        del args[k:k + 2]
    src, dst = Path(args[0]), Path(args[1])
    meta, body = front_matter(src.read_text(encoding="utf-8"))
    dst.write_text(page(meta, render_body(body), section), encoding="utf-8")
    print(f"{src} -> {dst}")


if __name__ == "__main__":
    main()
