# Wasm Apps: everything is an app (Lean, phase 2)

## 1. Summary, principles, non-goals

Phase 1 (`plans/lean-browser.md`) built a browser whose renderer never parses HTML: a short-lived loader compiles a web page into a read-only, memory-mapped **page file** (rkyv, `role`/`name` per node) and a small immediate-mode renderer paints it. Phase 2 keeps the renderer and the page file and changes what produces them. Instead of one loader compiling HTML, **every card on screen is an app**: a WebAssembly component (WASI 0.2, WIT) that emits a UI tree in a new layout language, which the host compiles into page-file nodes and paints. The legacy web is demoted to one such app: the phase-1 loader wrapped behind the same WIT world.

The spirit is Palm's webOS: cards, a service bus, shared data services (Synergy), and a global "Just Type" that dispatches to whichever app registered a handler. The difference from webOS (and from every browser since) is that JS and HTML/CSS are not the substrate. Apps are wasm; UI is a host-owned, versioned language; layout and painting are done once, by the host, in the phase-1 renderer.

### 1.1 Principles, stated as testable policy

**P1 – The only reasons to limit a feature are memory or network.** Every "no" in this document, in the layout language, and in the WIT world must cite one of two justifications, and each justification is checkable:

- *Memory*: the feature would raise the host's steady-state `Private_Dirty` (measured as in phase 1, §9) or a card's frozen/running budget (§7 here) beyond its gate, or would require memory proportional to something an app controls without a cap.
- *Network*: the feature would require bytes on the wire that are not content-addressed and cacheable, or would keep a connection open beyond the idle rules in §5.

A proposal to reject or restrict a feature with any other justification ("complexity", "taste", "security theatre") is out of policy. Security restrictions are allowed but must be phrased as capabilities the user can grant, not as absent features. This is enforced socially: the `DECISIONS.md` log in the workspace records the justification for each restriction, and CI fails if a `deny`/`unsupported` entry in the layout-language spec or WIT world lacks a `because: memory|network` tag.

**P2 – Memory-first, as in phase 1.** The target metric is unchanged: `Private_Dirty` of the long-lived processes. File-backed, read-only bytes are free; anonymous dirty pages are the cost. Everything is designed so that what an app *is* (code, assets, UI, saved state) lives in files and what an app *does* lives briefly in a capped, killable instance.

**P3 – Apps describe, the host renders.** No app has access to pixels. There is no canvas, no framebuffer, no font API. Apps produce trees; the host owns layout, text shaping, painting, accessibility, selection, find, zoom, and contrast. This is the mechanism that stops apps bundling a renderer (§3.4) and is justified under P1 by memory (one shaping context, one glyph cache, one layout engine per device instead of one per app).

**P4 – Offline by default; the network is a phase, not a state.** Apps run from local files and local data. Connecting is something the host does on the app's behalf, briefly, in batches.

### 1.2 Non-goals for phase 2

- Running Lean apps inside a conventional browser (a JS re-implementation of the renderer). The reverse bridge is a static snapshot, §5.6.
- Multi-window or desktop chrome. Cards are the only window model.
- A general-purpose CRDT library in the host (§5.4 explains what the host does offer).
- Any JIT. Interpretation plus optional ahead-of-time compilation only (§4.5).
- Non-Linux hosts before W6, as in phase 1.
- Backward compatibility of the layout language before W5; the language is versioned and may break between W-milestones.

## 2. The layout language: Lui

The language is called **Lui** ("Lean UI"), file extension `.lui`. It exists in three forms that are one tree: a **text form** (KDL), a **builder form** in each promoted stack's language, and a **wire form** (the compact node buffer an app hands to the host, §4.2). The text form is the specification; the other two are isomorphic to it.

### 2.1 What we are escaping and what we are borrowing

HTML/CSS's pain is not that it is declarative; it is that its rules are *global* and *implicit*: the cascade and specificity (any rule anywhere can change any node), inheritance of a long, inconsistent list of properties, margin collapsing, floats and clearance, `box-sizing` defaults, `display` values that change what children mean, and selectors that make styling depend on document structure at a distance.

The constraint-based models (SwiftUI, Flutter, Compose, Subform, Yoga/flexbox) agree on a better core: a node's size is negotiated between parent and child through a small, local protocol; layout is a tree, not a document; styling is a parameter, not a query. Flutter's "constraints go down, sizes go up, parent sets position" is the model Lui adopts because it is the simplest to implement with one pass and it is already what the phase-1 renderer does for flex/grid through taffy. Subform teaches the sizing vocabulary (`fit`/`fill`/fixed/fraction) and shows that it is enough. Yoga teaches what to avoid: when you keep CSS's names you keep CSS's expectations.

Lui's rules:

1. **No cascade, no selectors, no specificity.** A style applies to a node because the node names it. Names are lexically scoped (file, then component).
2. **One inherited thing.** The only value that flows down the tree is the *default text style token* (§2.4). No colour, font, or size property is inherited individually.
3. **No margins.** Spacing is `gap` on containers and `pad` on nodes. Margin collapsing cannot exist.
4. **Boxes are border-box.** Always.
5. **Position is a container's job.** Only `layer` places children on top of each other, and only with named anchors. There is no `absolute`, `float`, or `z-index`.
6. **Containers do not change the meaning of their children.** A `text` inside a `row` is the same node as a `text` inside a `col`.
7. **The host may re-run layout at any time from the tree alone.** No layout-dependent app logic (no "measure then decide") in the contract. Apps see width *buckets* (§2.7), never pixels.

### 2.2 Surface syntax: KDL

Lui's text form is KDL 2. Justification against the alternatives:

- *Indentation-based*: fragile in copy/paste and in generated code; no existing parsers in the stack languages.
- *Builder APIs only*: every app must go through code, so there is no neutral form for templates, static snapshots (§4.6), the HTML-compiler app's output, or tooling. Builders are provided *as well* (§3.1).
- *KDL*: node-based, so it is already a tree; typed positional arguments and `key=value` properties map exactly onto Lui's "element, args, props, children"; comments, slashdash (`/-`) for disabling a subtree, multi-line strings; parsers exist in Rust (`kdl`), C, Zig, Go, and are ~1,000 lines to write. KDL is not parsed at runtime (§3.1): the stack's build step compiles `.lui` files into the wire form, so no parser ships in an app. That is the memory justification for having a text form at all.

### 2.3 Elements and containers

**Containers** (children in braces):

| Node | Meaning | Props |
|---|---|---|
| `col` | vertical stack | `gap`, `align` (start/center/end/stretch), `justify` (start/center/end/between) |
| `row` | horizontal stack | same, plus `wrap` (bool; wrapped rows become a flow of lines) |
| `grid` | 2-D grid | `cols` (track list: `"1fr 2fr 24u"`), `rows`, `gap`; children take `col`/`row` (start, span) |
| `layer` | overlay; children stack in order | children take `anchor` (one of nine positions) and `inset` |
| `scroll` | scrollable viewport, vertical by default | `axis` (v/h), `key` (scroll position is restored by key) |
| `list` | virtualised vertical list | `source` (list id the app serves in windows, §4.2), `estimate` (row height in `u`), `key` |
| `adapt` | picks one child by width bucket | children are `at <min-width>` blocks |
| `def` | component definition (§2.5) | name, parameters |

**Leaves**:

| Node | Meaning | Notable props |
|---|---|---|
| `text` | a paragraph of one style; inline `span` children for runs | `style`, `lines` (max lines, ellipsis), `select` (bool, default true) |
| `image` | raster or vector asset by hash | `src` (asset hash or `blob:` id), `alt` (required, may be `""` for decorative), `fit` (contain/cover/fill) |
| `path` | vector drawing as data (subset of SVG path/rect/circle, fill/stroke) | `view` (viewBox), `alt` |
| `button` | activatable | `on` (handler id), `label`, `kind` (primary/secondary/plain) |
| `link` | activatable with a destination | `to` (a `lean://` or `https://` URL, or `app:route`) |
| `field` | text input | `value`, `on` (change handler), `kind` (text/password/search/number/email), `multiline`, `placeholder`, `label` |
| `check`, `radio`, `switch` | boolean/choice controls | `value`, `on`, `label`, `group` (radio) |
| `choice` | select from options | `options` (list of `option`), `value`, `on` |
| `slider` | numeric | `min`, `max`, `step`, `value`, `on` |
| `divider` | separator | `axis` |
| `spacer` | takes `fill` space | – |

There is no generic `box`/`div`. A container with one child is the box. The set is closed; adding an element requires a memory/network justification for why it cannot be a component built from these (P1 applies in both directions: a *missing* element must also be justified, since "we did not feel like it" is not a reason).

### 2.4 Sizing, styles, tokens

**Units.** One length unit, `u`, equal to 4 CSS px at zoom 1. Zoom (a user override, §4.7) scales `u`; nothing in an app needs to know. Fractions are `fr`; percentages of the parent's content box are `%`. Text sizes are not lengths: they are steps (§ below).

**Sizing.** Every node has `w` and `h`, each one of: `fit` (default; size to content), `fill` (take remaining space; `fill=2` is a weight), a number in `u`, or `n%`. `min-w`/`max-w`/`min-h`/`max-h` in `u`/`%`. The algorithm is Flutter's: parent passes min/max constraints, child returns a size, parent positions it. `fit` in a `scroll` axis means unbounded; the host clamps children to 1e6 u to keep layout finite (memory: bounded layout arrays).

**Styles.** A `style` node declares a named bundle. Nodes reference it with `.name` as the first positional argument. A node may name exactly one style plus inline props; inline props win. Styles are lexically scoped: file-level `style` nodes are visible in that file; `style` nodes inside a `def` are visible only in that component. There is no `extends`; composition is done by tokens.

Properties a style may hold (the complete list, 27 entries; see R1 in §9 for the growth rule): `pad` (1–4 values), `gap`, `bg` (colour), `fg` (colour of text and icons), `border` (width, colour), `radius`, `opacity`, `align`, `justify`, `w`, `h`, `min-w`, `max-w`, `min-h`, `max-h`, `text` (text token), `weight` (regular/bold), `italic`, `underline`, `strike`, `mono`, `wrap` (normal/none/pre), `lines`, `font-step`, `line-step`, `letter` (tracking, in 1/100 em), `fit`.

**Tokens.** `tokens { ... }` at file scope declares named values; `$name.path` references them. The host defines the system token set `$sys.*`: `$sys.color.{bg,fg,accent,muted,danger,link,selection}`, `$sys.space.{xs,s,m,l,xl}` (1,2,3,4,6 u), `$sys.text.{xs,s,m,l,xl,title}`, `$sys.radius.{s,m}`. Apps may define their own tokens, and may use literal colours, but:

- **Text sizes exist only as steps** (`font-step` −2…+6 relative to the user's base size), never as lengths. This is what makes the user's font-size override work everywhere.
- The host's **forced-colours mode** ignores every app colour and paints from `$sys.color.*`. Apps that used `$sys` tokens look right; apps that used literals look monochrome but legible. This mirrors Windows High Contrast and is the contrast override in the day-one contract.

**The single inherited value.** Any container may set `text-default=".name"`; every `text` below it without an explicit style uses that style. This is a single token, not per-property inheritance, so there are no partial-inheritance surprises: a `text` either has the named style or it has the inherited default, never a merge of the two.

### 2.5 Components, state, events

`def Name(param, param="default") { ... }` declares a component; `Name param=value { children }` instantiates it. The `@children` node inside a `def` is the slot. Parameters are typed by first use (string, number, bool, list, node). Components have no state of their own: all state is in the app, and a component is a pure function from parameters to nodes. This is Elm's architecture: the app holds a model, `render(model)` produces the whole tree, events produce messages, `update(model, message)` produces a new model. The choice is deliberate: TEA has no retained widget objects in the app, so the app's linear memory stays proportional to the model, not to the UI (memory justification), and it makes freeze/thaw trivial (the model is the state).

Control flow inside a template: `if cond { } else { }`, `for item in list { }` with `key=item.id`. Expressions are a tiny language (field access, comparison, `and`/`or`/`not`, string interpolation `"{item.subject}"`); no function calls, no arithmetic beyond `+`/`-` on numbers. Anything more is done in the app before rendering.

**Events.** Every interactive node has `on=<handler>` where `<handler>` is a `u16` the stack's compiler allocates per named handler in the app; the app's `update` receives `(handler, node-key, payload)`. Payloads are the enum in §4.2. **Keys.** Any node may carry `key=` (string or integer); the host uses keys to preserve scroll position, focus, selection, and input state across renders, and to compute deltas. `for` loops require keys.

**Input state is owned by the host.** A `field` node has a `value` the app *proposes*; what the user is typing lives in the renderer's input-state table (§2.9) and is reported through `change` events. The app's next render either echoes it (controlled) or omits `value` (uncontrolled). This keeps keystrokes from round-tripping through wasm on every character (memory: no thawing a frozen card to type in it; also CPU).

### 2.6 Text and rich text

`text` holds a paragraph; its children are `span`s and inline `link`s with their own styles. Text properties are explicit per span; no cascading. Paragraph-level: `align`, `lines`, `wrap`, `select`. The host shapes with swash exactly as in phase 1. Markdown-like content (a blog article) is a sequence of `text`/`image`/`path` nodes generated by the app or by the HTML-compiler app; there is no "rich text document" node, because it would be a second layout engine (memory).

**Selection, copy, find** are host features across all cards: because `text` nodes carry their runs in the page file's text blob, selection is a range over `(node key, byte offset)` pairs and is app-invisible. Apps may mark `select=false` on decorative text. Copy yields plain text plus, where spans carry `link to=`, a text/uri-list.

### 2.7 Responsive rules

Apps never see the viewport in pixels. The host exposes a width **bucket** (the phase-1 breakpoints, standardised: `compact` < 150 u, `medium` < 300 u, `expanded` otherwise) and a height bucket. Two mechanisms:

- `adapt { at 0 {...} at 150 {...} at 300 {...} }` picks the last `at` whose minimum is ≤ the container's width, resolved by the host at layout time. This is the common case and needs no app re-render.
- The `env` record passed to `render` carries the buckets; the app may branch on them. The host calls `render` again only when a bucket changes, mirroring phase-1's "re-cascade on breakpoint crossing".

### 2.8 Animation

Two host-provided, declarative effects and nothing else: `enter="fade|slide-up"` and `exit="fade"` on a keyed node (the host cross-fades when a key appears or disappears between renders, 150 ms, drawn by re-painting the strip buffer with opacity), and `scroll-to=key` on a `scroll`. No app-driven per-frame animation, because it would require keeping instances hot and running at frame rate (memory: prevents freezing; also CPU/battery), and no transitions on arbitrary properties because they require retaining two layout trees (memory). The effects are skipped entirely under the "reduce motion" override.

### 2.9 Compilation to the page file, and the changes phase 1 needs

The host compiles a Lui tree into a phase-1 `Page`: each Lui node becomes a `Node` with `kind` from a new `NodeKind` range (`LuiCol = 96`, `LuiRow`, `LuiGrid`, `LuiLayer`, `LuiScroll`, `LuiList`, `LuiText`, `LuiSpan`, `LuiImage`, `LuiPath`, `LuiButton`, `LuiLink`, `LuiField`, `LuiCheck`, `LuiRadio`, `LuiSwitch`, `LuiChoice`, `LuiSlider`, `LuiDivider`, `LuiSpacer`), `role` and `name` set from the element and its `label`/`alt` (the day-one accessibility rule holds by construction: the compiler refuses an `image` without `alt` and a `button`/`field` without a `label` or text child), and `style` pointing at an interned `ComputedStyle`. Lui's `col`/`row`/`grid` map onto the existing flex/grid enums (taffy); `layer` maps to a new `Position::Anchored` with the anchor in the inset fields; `pad`/`gap`/`border`/`radius`/`bg`/`fg` map to existing properties. Nothing new is needed in `ComputedStyle` except `Anchored`, `lines` (max lines) and `font_step`.

**Changes to the phase-1 page format** (`crates/page-format`, bump `FORMAT_VERSION`):

1. `Node.reserved: u8` becomes `handler_lo: u8` and a new parallel table `Page.handlers: Vec<Handler { node: u32, handler: u16, events: u8 }>` sorted by node (keeps `ArchivedNode` at 32 bytes; the byte is a "has handler" flag).
2. `Page.keys: Vec<Key { node: u32, key: u64 }>` sorted by node; keys are 64-bit hashes of the app's key.
3. `Page.scrolls: Vec<ScrollContainer { node: u32, axis: u8, key: u64 }>`; the renderer keeps a `heights[]`-style array per scroll container, capped as in phase 1 (§7 of plan 1). Phase 1 rejected inner scrollbars for HTML; Lui requires them, bounded to 64 scroll containers per page (memory: 64 × ≤ 64 KB `heights[]` worst case, typically a few hundred bytes).
4. `Page.lists: Vec<ListSource { node: u32, source: u16, estimate: u16, count: u32 }>`: the page file holds only the *window* of rendered rows; the renderer asks the app host for more rows on scroll (§4.2).
5. `Page.inputs: Vec<InputInit { node: u32, kind: u8, value_off: u32, value_len: u32 }>`: proposed values. The renderer's existing `form_state` vector becomes the **input-state table** keyed by node key, surviving re-renders.
6. `Page.app: Option<AppHeader { app_id: [u8;32], generation: u32, route_off: u32, route_len: u32 }>`: which app, which render generation, and the deep-link route this page represents (§4.6).
7. **Deltas.** A page file is immutable. Between full rewrites, the host appends to a sibling delta file `page.lpd` (postcard-framed ops): `SetText { key, off, len }` (text appended to a delta text blob), `SetStyle { key, style }`, `SetValue { key, value }`, `ListWindow { source, start, rows: Vec<Node>... }`, `ReplaceSubtree { key, nodes... }`. The renderer applies deltas at layout time from a small in-memory overlay (cap 64 KB dirty). When the overlay exceeds the cap or 256 ops, the host compacts: writes a new `page.lpg` and truncates the delta. This is the phase-1 M6 "rewrite incrementally" mechanism, made concrete.

**Changes to the renderer**: multiple mapped pages (one per visible card, plus frozen cards mapped lazily); inner scroll containers; selection and copy (phase 1 had only find); the event path from a click on a node with a handler to the app host (§4.3) instead of to the loader; keyed input state; forced-colours paint mode; `enter`/`exit` cross-fade.

### 2.10 Examples

**List-detail mail screen** (one file, two components, list virtualised):

```kdl
tokens {
  color.unread "$sys.color.accent"
}
style "subject" text="$sys.text.m" weight="bold" lines=1
style "preview" text="$sys.text.s" fg="$sys.color.muted" lines=2
style "row" pad="$sys.space.m" gap="$sys.space.xs" radius="$sys.radius.s"
style "row-selected" pad="$sys.space.m" gap="$sys.space.xs" radius="$sys.radius.s" bg="$sys.color.selection"

def MessageRow(msg, selected=false) {
  button (if selected ".row-selected" else ".row") on=open key=msg.id label="{msg.from}: {msg.subject}" {
    col gap="$sys.space.xs" {
      row gap="$sys.space.s" align="center" {
        if msg.unread { path view="0 0 8 8" alt="unread" w=2 h=2 { circle cx=4 cy=4 r=4 fill="$color.unread" } }
        text ".subject" w="fill" { "{msg.from}" }
        text ".preview" { "{msg.when}" }
      }
      text ".subject" { "{msg.subject}" }
      text ".preview" { "{msg.preview}" }
    }
  }
}

def Detail(msg) {
  scroll key="detail-{msg.id}" {
    col pad="$sys.space.l" gap="$sys.space.m" text-default=".body" {
      text text="$sys.text.title" { "{msg.subject}" }
      row gap="$sys.space.s" { text ".preview" { "{msg.from}" } spacer; text ".preview" { "{msg.when}" } }
      divider
      for block in msg.body key=block.id { LuiBlock block=block }   // body blocks rendered via a Rich-text-like component, elsewhere
      row gap="$sys.space.s" {
        button label="Reply" on=reply kind="primary"
        button label="Archive" on=archive
      }
    }
  }
}

adapt {
  at 0 {                       // compact: one pane at a time
    if model.open { Detail msg=model.open } else {
      list source=inbox estimate=18 key="inbox" { for msg in window key=msg.id { MessageRow msg=msg } }
    }
  }
  at 200 {                     // medium and up: two panes
    row {
      list source=inbox estimate=18 key="inbox" w=80 { for msg in window key=msg.id { MessageRow msg=msg selected=(msg.id == model.open.id) } }
      divider axis="v"
      if model.open { Detail msg=model.open } else { col justify="center" align="center" w="fill" { text ".preview" { "Select a message" } } }
    }
  }
}
```

**Blog article** (as the HTML-compiler app emits it from the phase-1 Zola blog, or as a native blog app writes it):

```kdl
style "h1" text="$sys.text.title" weight="bold"
style "h2" text="$sys.text.xl" weight="bold"
style "body" text="$sys.text.m"
style "meta" text="$sys.text.s" fg="$sys.color.muted"
style "code" text="$sys.text.s" mono=true wrap="pre" pad="$sys.space.m" bg="$sys.color.bg" border="0.25 $sys.color.muted" radius="$sys.radius.s"

scroll key="post-{post.slug}" {
  col align="center" {
    col max-w=180 w="fill" pad="$sys.space.l" gap="$sys.space.m" text-default=".body" {
      text ".h1" { "{post.title}" }
      text ".meta" { "{post.date} · {post.reading_time} min" }
      text { "Lean Browser is a limited web browser whose " span weight="bold" { "single optimizing target" } " is minimal resident private memory." }
      text ".h2" { "Process model" }
      image src="blake3:9f3c…" alt="Two boxes, loader and renderer, joined by a pipe" w="fill" fit="contain"
      scroll axis="h" key="code-1" { text ".code" select=true { "#[repr(C)]\npub struct Header { … }" } }
      text { "See " link to="lean://developmeh.com/blog/lean-browser" { "the plan" } " for details." }
    }
  }
}
```

## 3. Promoted stacks

### 3.1 What a stack is

A **stack** is the host-blessed way to write an app in one language. It consists of:

1. **The Lui compiler** for that language's build: `.lui` files become a wire-form emitter (Rust: a proc-macro `lui!{}` plus `build.rs`; C/Zig: a `luic` CLI emitting a `.c`/`.zig` table). Output is straight-line code that appends nodes to a buffer. No KDL parser, no template interpreter in the binary (memory and network: a parser plus its tables is 50–150 KB of code per app).
2. **The SDK**: the TEA loop (`Model`, `update`, `render`), the wire-form buffer builder, list-window serving, a `Store` client over the storage WIT, and the sync client bindings. The SDK is also where "save-state" gets a default implementation: serialise the model with a schema-versioned encoder (Rust: `postcard`).
3. **The WIT bindings**: `wit-bindgen` output for `lean:app` (§4), pre-generated and pinned to the stack version.

### 3.2 Languages, in order

| Order | Language | Why | Concern |
|---|---|---|---|
| W1 | **Rust** | `wit-bindgen` and `cargo-component` are native; `no_std`+`alloc` apps compile to 50–200 KB modules; the host and the HTML-compiler app are Rust, so the first stack is dogfooded. | none |
| W2 | **C** (and **Zig** through the C ABI) | `wit-bindgen c` is mature; smallest modules of all; Zig's `wasm32-wasi` support is good. | Zig lacks a native component toolchain; use `wasm-tools component new` on core modules. |
| W4 | **TinyGo** | Component-model support exists (`-target=wasip2`), large existing ecosystem. | Runtime is ~500 KB and GC needs headroom; the per-card memory cap will bite. Promoted only if a hello-world card fits the 4 MB "small" class (§4.4). |
| Later | **AssemblyScript** | Attractive for JS developers. | No component-model bindings yet; wait. |
| Never as a stack | JS engines | A QuickJS-in-wasm stack would work but costs 1–2 MB per running card. Rejected on memory grounds (P1). Revisit if a shared-module JS engine with per-app heaps proves under 512 KB per card. |

### 3.3 Versioning, content addressing, sharing

A stack's runtime part (the SDK's compiled code) ships as a **wasm library component** identified by `lean:stack-rust@1.4.0` and its BLAKE3 hash. An app's manifest names the stack and the hash. At install time the host resolves the stack from its own content-addressed store (or from the stack registry, §5.1) and **composes** the app with it (`wasm-tools compose` semantics, done by the host at instantiation, not by the app's build). Result:

- The stack's `Module` is compiled and validated by wasmi **once** per host and shared by every instance (code pages are per-module, instance memory is per-app). This is the sharing that matters for memory.
- Two apps on the same stack version fetch the stack once (network).
- The app's own module contains only app logic. A Rust app that inlines the SDK instead (static linking) is allowed but its bytes count against its own code budget (§3.4), which is the incentive to use the shared module.

Version policy: stacks are semver; the host keeps the newest patch of each minor it has seen an app pin, and evicts stack versions no installed app references. An app pins an exact hash; "upgrade the stack" is a new app release. No floating versions, because floating versions break content addressing.

### 3.4 How apps are prevented from bundling their own renderer, on memory/network grounds

There is no capability to draw. The `lean:app/ui` interface accepts trees; the only bitmap-like thing an app can produce is an `image` from an asset or a `blob:` it fills through `ui.blob-put` (capped at 256 KB per blob, 2 MB per card, justified as image-scratch memory in phase 1 §7). A "renderer inside the app" would therefore have to rasterise into blobs and show them as images. To make that a losing strategy rather than a forbidden one:

- Module size cap: 2 MB per app module (network justification: apps are fetched over possibly poor links; the cap is P1-compliant and stated in the manifest schema). Shared stacks do not count.
- Blob cap above; a blob-based framebuffer at 200 × 300 u is over the cap.
- Blobs are not deltas: every `blob-put` is a full replace and forces a page-file compaction. An app that "renders" this way freezes badly, thaws slowly, and looks wrong under user overrides. Nothing else stops it. That is the honest answer under P1.

## 4. App model and WIT world

### 4.1 Processes

```
renderer (phase 1, trusted)          app host (new, trusted code, untrusted guests)
  mmap page.lpg per card       <----->  wasmi engine; one Module cache; one Instance per running card
  input-state, selection, a11y   IPC     lifecycle, freeze/thaw, capability broker
  events -> app host                       |
                                           v
sync broker (new, network-facing)     content store (on disk, hashed, shared)
  QUIC / HTTPS fallback ladder
  per-origin connection multiplexing
```

The app host is one process (seccomp: no `socket`; Landlock: content store read, state dir read/write). All guests share it because wasm gives memory isolation; the process boundary is for host bugs, and a second process per app would cost ~300–500 KB dirty each (measured for a musl static binary in phase 1's M0; estimate). Two apps are **host-native**: the HTML-compiler app (the phase-1 loader, running as its own sandboxed process exactly as before, but speaking `lean:app` over IPC) and the shell (cards, Just Type, launcher). They implement the same WIT world; the host does not special-case them beyond granting privileged capabilities.

### 4.2 The WIT world

```wit
package lean:app@0.1.0;

interface types {
  type handler = u16;
  type key = u64;                       // hash of the app's node key
  type card = u32;

  record env {
    width-bucket: u8, height-bucket: u8,   // §2.7
    zoom-step: s8, forced-colors: bool, reduce-motion: bool,
    locale: string, offline: bool,
  }

  variant payload {
    activate,                            // button, link
    change(string),                      // field
    toggle(bool),                        // check, switch
    select(string),                      // radio group, choice
    number(f32),                         // slider
    submit,                              // Enter in a field
    scrolled(u32),                       // scroll: first visible key's index (list)
  }

  record event { handler: handler, key: key, payload: payload, at: u64 /* ms, coarse */ }

  record intent { verb: string, query: string, mime: option<string>, data: option<list<u8>> }
}

interface ui {
  use types.{key, card};
  // Wire form: a length-prefixed buffer of nodes in pre-order, each
  // {kind: u8, flags: u8, style: u16, key: u64, handler: u16, text-or-src: (off,len), children: u16}
  // followed by a style table and a text blob. The stack SDK builds it; apps never see the layout.
  submit: func(tree: list<u8>);        // full tree; host diffs against the previous generation
  patch: func(ops: list<u8>);          // §2.9 delta ops, for targeted updates
  blob-put: func(id: u32, bytes: list<u8>, mime: string) -> result<_, string>;
  set-title: func(title: string);
  set-route: func(route: string);      // current deep-link route, §4.6
}

interface store {
  // Offline-first per-app storage (§5.4). Keys are bytes; values are bytes.
  record version { device: u32, seq: u64 }
  record entry { key: list<u8>, value: list<u8>, version: version }
  get: func(key: list<u8>) -> option<entry>;
  siblings: func(key: list<u8>) -> list<entry>;          // concurrent versions, if any
  put: func(key: list<u8>, value: list<u8>, parent: option<version>) -> version;
  delete: func(key: list<u8>, parent: version);
  scan: func(prefix: list<u8>, after: option<list<u8>>, limit: u32) -> list<entry>;
  log-append: func(channel: string, op: list<u8>) -> version;
  log-read: func(channel: string, after: option<version>, limit: u32) -> list<tuple<version, list<u8>>>;
}

interface sync {
  enum wish { now, soon, idle }        // now: user is waiting; soon: within the batch window; idle: next wake window
  request: func(channel: string, wish: wish);
  record status { channel: string, connected: bool, pending: u32, last-ok: option<u64> }
  status: func(channel: string) -> status;
}

interface fetch {
  // Capability-gated. Host-side allow-list of origins from the manifest.
  record request { url: string, method: string, headers: list<tuple<string,string>>, body: option<list<u8>> }
  record response { status: u16, headers: list<tuple<string,string>>, body: list<u8> }
  send: func(req: request) -> result<response, string>;   // 4 MB body cap, batched with sync (§5.3)
}

interface bus {
  // Service bus between apps. Publish/subscribe on typed topics; calls are routed through the host.
  call: func(service: string, method: string, args: list<u8>) -> result<list<u8>, string>;
  publish: func(topic: string, data: list<u8>);
}

interface guest {
  use types.{env, event, intent, key};
  init: func(env: env, route: option<string>, state: option<list<u8>>);   // fresh start or thaw
  render: func(env: env);                                                  // must call ui.submit
  update: func(events: list<event>);                                       // batched; host calls render after
  rows: func(source: u16, start: u32, count: u32);                         // serve a list window via ui.patch
  save-state: func() -> list<u8>;                                          // <= 256 KB; called before freeze
  handle-intent: func(intent: intent) -> option<string>;                   // returns a route to open, or none
  snapshot: func(env: env, route: string);                                 // static render, no state, for indexing
  serve: func(service: string, method: string, args: list<u8>) -> result<list<u8>, string>;   // bus provider
}

world app {
  import wasi:clocks/monotonic-clock@0.2.0;   // coarsened by the host to 100 ms
  import wasi:random/random@0.2.0;
  import types; import ui; import store; import sync; import fetch; import bus;
  export guest;
}
```

Not imported: `wasi:filesystem`, `wasi:sockets`, `wasi:http`. Apps have no files and no sockets; storage is the `store`, the network is the host's sync service and gated `fetch`. This is the capability model in one line.

### 4.3 Event path

Renderer hit-tests the click on a node with a handler entry, looks up `(handler, key)`, and sends `Event` to the app host. If the card is running, the host batches events for up to 16 ms and calls `update` then `render`; `submit`'s buffer is diffed against the previous wire-form (kept on disk, clean) into a delta or a new page file; the renderer is told `Generation { card, page_path, delta_len }` and repaints. If the card is frozen, the host queues the event, thaws (§4.4), and delivers. Typing, scrolling, selection and find never reach the app unless a handler subscribes.

### 4.4 Card lifecycle and memory caps

```
          install                      open
 (absent) ------> cold (files only) --------> running
                    ^                          |   ^
                    | evict (user closes)      |   | thaw complete
                    |               freeze     v   |
                  frozen  <---------------- running    frozen --(event or focus)--> thawing --> running
```

- **cold**: manifest, module, assets in the content store; no state. 0 dirty bytes.
- **running**: a wasmi instance with a hard linear-memory cap from the manifest's `memory` class: `small` 4 MB, `medium` 8 MB, `large` 16 MB (max; larger requires a user-visible grant). `memory.grow` beyond the cap traps. The host also enforces a **global** running budget (default 48 MB of guest memory across all cards); exceeding it freezes the least-recently-interacted card first.
- **freeze**: triggered by the global budget, by 60 s of background inactivity, or by the system's low-memory signal. Host calls `save-state`, writes the blob (≤ 256 KB) to the state dir, keeps the last `page.lpg` (clean, file-backed) and `page.lpd`, drops the instance. Wasm linear memory never shrinks, which is why the instance must be dropped rather than "paused".
- **frozen**: the card is fully browsable: scroll, select, copy, find, zoom, a11y all work from the page file. Cost is ~1 KB of host bookkeeping plus the mapping (clean).
- **thawing**: on an event or focus, the host shows the frozen page immediately, instantiates in the background (`init` with the saved state), calls `render`, diffs; if the tree is identical (usual case) nothing repaints. Events received while thawing are queued; if thaw takes more than 300 ms the card shows a thin progress line. Budget: thaw of a `small` card ≤ 150 ms on the reference device (§8, W3 exit).
- **kill**: an instance that traps, exceeds a 2 s CPU slice per `update`/`render` (wasmi fuel metering), or exceeds memory is dropped and the card shows the last page with a "restart" affordance. The state blob from the last successful freeze is what it restarts from.

### 4.5 Runtime choice and the JIT question

**wasmi** (pure Rust, register-based interpreter since 0.31, `no_std`, supports the component model through `wasmi` 0.4x's component support or, until that lands, through the host's own canonical-ABI shim over core modules; W1 verifies which). Chosen over WAMR because the app host is Rust with `forbid(unsafe_code)` outside two crates, and over Wasmtime because Cranelift plus its compiled-code caches cost tens of megabytes dirty. wasmi's per-instance overhead is a few tens of KB plus the linear memory (estimate).

**JIT: no.** A JIT produces anonymous, dirty, executable pages per instance, cannot be shared, and cannot be frozen to disk cheaply. **AOT: maybe, later.** Ahead-of-time compilation (WAMR's `wamrc`, or a future wasmi backend) produces file-backed native code that is `Private_Clean` — the memory-friendly form of speed. Trigger to reconsider (§9, R3): if W4 profiling shows the HTML-compiler app or a real app exceeding its 2 s CPU slice on the reference device in interpreted mode for common operations. Until then, interpretation with fuel metering is the runtime.

### 4.6 Manifest, deep links, snapshot

`manifest.kdl`, signed (§6):

```kdl
app "com.example.mail" version="1.3.0" name="Mail" {
  entry "blake3:…"                       // app component
  stack "lean:stack-rust@1.4.0" hash="blake3:…"
  memory "medium"                        // 8 MB cap
  assets { "icons.lpk" "blake3:…"; "hero.jpg" "blake3:…" }
  routes { "/" "inbox"; "/m/{id}" "message"; "/compose" "compose" }
  intents { verb="compose" mime="text/plain"; verb="search" }
  channels { "inbox" mode="log" scope="user"; "settings" mode="kv" scope="user" }
  capabilities { fetch origins="api.example.com"; bus provide="contacts.read"; bus use="contacts.read"; notify }
  snapshot routes="/"                    // routes rendered statically for indexing
  signature key="ed25519:…" sig="…"
}
```

**Deep links** are in the contract: every `lean://host/app-id/<route>` maps to a manifest route; `init` receives the route; `ui.set-route` reports the current route so the shell can share/bookmark it and so the `page.lpg`'s `AppHeader.route` is always right. **Snapshot**: the host calls `guest.snapshot(env, route)` for each declared snapshot route at install time and on each app update, producing static `page.lpg` files that the shell's search, the HTTP gateway (§5.6) and any indexer read without instantiating the app. Snapshots are the discovery surface.

### 4.7 User overrides

Zoom (scales `u`), base font size (shifts steps), forced colours, reduced motion, font face (swap the three bundled faces for user-chosen ones), minimum tap target (host inflates hit boxes). All are renderer-side and require no app cooperation; a bucket change from zoom triggers `render` as in §2.7.

## 5. Networking and sync

### 5.1 `lean://` over QUIC

URL: `lean://<host>[:port]/<app-id>[/<route>]`. Connection: QUIC (quinn) with ALPN `lean/1`, TLS 1.3, 0-RTT resumption for known origins. All requests are client-initiated bidirectional streams with one frame type per stream:

```
frame := type u8 | flags u8 | len u32 | payload
types:  MANIFEST(name, version-hint) -> signed manifest bytes
        BLOB(hash) -> bytes (content-addressed; immutable; may be served by any origin or the stack registry)
        SYNC-OPEN(channel-set, versions) -> bidi stream of log entries / kv ops in both directions (§5.3)
        FETCH(request) -> response (gated HTTP through the origin's gateway, §5.5)
```

Content addressing: BLAKE3, 32 bytes. Because blobs are immutable, the content store caches them **forever** subject to an LRU size cap (default 256 MB). This reverses phase 1's "no cache" non-goal, explicitly: app code and assets are public, immutable, and not per-user; browsing state still is not cached. Privacy mitigation: shared stacks are fetched from the stack registry, not the app origin, so the origin learns nothing about the other apps installed; cache hits are never observable to apps (all `BLOB` latency is reported coarsely).

Manifests are the only mutable thing: `MANIFEST` requests carry the installed version and are answered `304`-style when unchanged. Updates are atomic (a new manifest names new hashes; old cards keep their pinned hashes until restarted).

### 5.2 The sync broker and connection policy

The sync broker owns every socket in the system. Apps declare channels in the manifest; the broker **multiplexes all channels of all apps that share an origin over one QUIC connection**, opens it when there is work, and closes it when idle:

- Batching: `wish=now` sends immediately; `wish=soon` coalesces for 2 s (foreground) or until the next wake window (background); `wish=idle` waits for the next wake window.
- Wake windows: every 15 min while on battery and backgrounded, aligned across origins so the radio wakes once; every 2 min in foreground; continuous (connection held) only while a foreground card has an open `SYNC-OPEN` stream with `live=true` and the user has interacted within 5 min.
- Idle disconnect: QUIC `max_idle_timeout` 30 s; the broker sends nothing to keep it alive. 0-RTT makes reopening cheap.
- Backoff: exponential from 5 s to 1 h per origin with jitter; a `now` wish resets it.
- Metered/low-battery flags from the OS: batch windows stretch to 1 h; `fetch` bodies over 512 KB are deferred unless `now`.

### 5.3 Transport fallback ladder

Preference and detection, per origin, remembered for the session and re-probed hourly:

1. **QUIC** (UDP 443 or the URL's port). Downgrade if no handshake within 3 s, or two consecutive connection failures, or the network reports UDP blocked (ICMP admin-prohibited).
2. **HTTPS + SSE**: `POST /lean/sync` for client→server batches; `GET /lean/events?channels=…` as a server-sent-event stream for server→client. Downgrade if the stream yields no `:ping` within 20 s of connecting (buffering proxy) or the response is not `text/event-stream`.
3. **WebSocket** over HTTPS: same message framing, both directions on one socket. Downgrade on 4xx upgrade failure or two dropped connections within a minute.
4. **Polling**: `GET /lean/sync?after=<version-vector>` with `If-None-Match`, long-poll up to 25 s if the gateway supports it, else short poll at the wake-window cadence. Never downgraded from; upgraded on the hourly re-probe.

All four carry the same framed messages; only 1 supports multiple streams per connection natively, so 2–4 multiplex by message tagging.

### 5.4 Offline-first data model

The host provides two primitives per channel, both persisted in the app's state dir (SQLite would cost ~300 KB dirty per open connection; instead: an append-only log file per channel plus a memory-mapped index, compacted by the broker in the background — estimate ≤ 32 KB dirty per open channel):

- **Log** (`mode="log"`): append-only entries stamped `(device, seq)`; the channel's state is a **version vector** over devices. Sync exchanges "entries you have not seen" in both directions. Apps that need CRDT semantics implement op-based CRDTs on top: a log of commutative ops *is* the CRDT; the host guarantees causal delivery order per device and exactly-once. The host does **not** ship a general CRDT library (memory: Automerge-style metadata is several times the payload). A text CRDT is planned as a *stack library* (W6), not host code.
- **KV** (`mode="kv"`): last-writer-wins by version vector with **siblings** kept when writes are concurrent; `store.siblings` returns them, the app resolves and `put`s with both as parents. The SDK offers a default resolver (latest device clock wins) for apps that do not care.

Scopes: `user` (synced across the user's devices via the origin), `device` (never synced), `shared:<group>` (W7, multi-user). Conflicts never block sync; they are data.

### 5.5 `fetch` and the HTTP gateway

Apps cannot open HTTP themselves; `fetch.send` is proxied through the origin's gateway (`FETCH` frame or `POST /lean/fetch` in fallback), which enforces the manifest's origin allow-list. This keeps every app's traffic on the single per-origin connection and lets the broker batch it. Cost: the gateway sees the app's requests; the trade-off is documented and apps that need direct third-party access request `fetch origins="*"` which the user must grant.

### 5.6 The gateway and the reverse bridge

The HTTP gateway (`https://<host>/lean/…`) is a small server the origin runs (a reference implementation ships in Rust with the broker crate): it serves manifests and blobs over HTTPS, implements the fallback endpoints of §5.3, and **serves snapshots as HTML** for normal browsers: `GET https://<host>/<app-id>/<route>` with a browser `Accept` header returns the snapshot page file rendered to static HTML by a host-side "page-to-HTML" pass (the inverse of the HTML-compiler app, trivial because Lui is a strict subset of what CSS flex/grid can express), with a link to open the app in Lean and `Link: <lean://…>; rel="alternate"`. That is the whole reverse bridge. Running live Lean apps in browsers is a non-goal (§1.2).

## 6. Security and privacy

- **Sandbox**: wasm memory isolation per instance; the app host process under seccomp (no `socket`, no `execve`) and Landlock; the broker is the only process with sockets, as the phase-1 broker was. wasmi fuel bounds CPU; memory caps bound RAM. Guest traps are contained to the card.
- **Capabilities**: everything in §4.2 beyond `ui`, `store` (own scope) and `wasi:clocks/random` is a manifest capability: `fetch` (per origin), `bus provide/use` (per service name), `notify`, `sync live`, memory above `medium`. The shell prompts at install for the list and at first use for anything marked `ask`. Grants are per app version hash; a new version re-asks only for *new* capabilities.
- **Signing**: manifests are signed by an Ed25519 developer key. Key-to-origin binding: `https://<host>/.well-known/lean-keys` (TOFU-pinned on first install, rotation requires the old key to sign the new). Every blob is verified against its hash before it touches the store.
- **Stack supply chain**: promoted stacks are published to a registry with an append-only transparency log (Sigstore/Rekor-style; W5 uses a simple signed Merkle log served by the registry). The host ships a built-in list of promoted stack hashes and accepts stack updates only if they appear in the log and are signed by the stack maintainers' key. Apps pin exact hashes; the host never substitutes.
- **Service bus**: a `bus.call` is delivered only if the caller's manifest `use`s the service and the provider's manifest `provide`s it and the user granted both; payloads are opaque bytes with a MIME tag; the host adds the caller's app id, never the user's identity.
- **Fingerprinting**: apps see width/height buckets, locale, coarse time (100 ms), and a per-(app, origin) random device id used only for version vectors, regenerated on uninstall. No fonts list, no hardware, no exact viewport, no other apps' presence, no cache timing (all blob loads report coarse durations). This is stricter than phase 1 because apps are longer-lived than pages.
- **Privileged host-native apps** (shell, HTML compiler) run under phase-1's loader sandbox and their capabilities are enumerated in the host's own `DECISIONS.md`, not a manifest.

## 7. Memory budget (all figures are estimates until measured; see W0)

| Component | Target | Notes |
|---|---|---|
| Renderer (phase 1) | ≤ 2.5 MB median / 4 MB max | unchanged gate; multi-card mappings are clean |
| App host baseline | ≤ 1.5 MB | wasmi engine, module cache (compiled modules are dirty in wasmi: budget ≤ 512 KB for the stack + shell modules; investigate `Module` serialisation to disk in W1) |
| Sync broker baseline | ≤ 1.0 MB | quinn + rustls; one connection ≤ 128 KB; four connections max concurrent |
| Content store index | ≤ 128 KB | mmap'd hash index, clean; dirty only the write buffer |
| **Per running card, `small`** | ≤ 4 MB guest + ≤ 96 KB host | linear memory (cap), wasmi instance (~48 KB), stack instance memory (~128 KB, inside the cap), previous wire-form buffer on disk (clean), delta overlay ≤ 64 KB |
| Per running card, `medium`/`large` | ≤ 8 / 16 MB guest | same host overhead |
| **Per frozen card** | ≤ 4 KB dirty | bookkeeping + input-state entries; `page.lpg`/state blob on disk, mapped clean on demand |
| Per open channel | ≤ 32 KB | log tail buffer + index write buffer |
| Global guest budget | 48 MB default | freeze LRU above it; configurable |
| **Typical: 12 open cards, 3 running** | ≤ 5 MB host + ≤ 12 MB guest | the webOS "too many cards" case: 9 frozen cards cost ≤ 36 KB |

CI gates: renderer unchanged; app host + broker ≤ 3 MB with zero cards; frozen card marginal cost ≤ 8 KB (measured by freezing 100 cards and dividing).

## 8. Milestones (continuing phase 1's M0–M7)

**W0 – Contract and baselines (2 weeks).** Page-format changes §2.9 (`FORMAT_VERSION` bump, new tables, delta file), WIT package `lean:app@0.1.0` checked in with `wit-bindgen` CI, `luic` KDL→wire-form compiler (Rust crate, CLI), Lui spec v0 with the 27-property list and the P1 justification log. Exit: phase-1 corpus still passes; `luic` round-trips both §2.10 examples to page files that the phase-1 renderer paints (static, no app); app host and broker skeleton processes measured at ≤ 3 MB combined `Private_Dirty`.

**W1 – Rust stack and first running card (4 weeks).** wasmi in the app host; component-model shim; `lean-stack-rust` with TEA loop, `lui!{}` macro, default `save-state`; the mail example as a real app with in-memory data; event path renderer→host→guest→delta→renderer. Exit: mail app runs, `small` class, ≤ 4 MB guest; end-to-end click-to-repaint ≤ 50 ms on the reference device; host overhead per running card ≤ 96 KB measured.

**W2 – Card lifecycle (3 weeks).** Freeze/thaw state machine, global budget with LRU freeze, fuel metering and kill/restart, input-state table keyed by node key, inner scroll containers, virtualised `list` with `rows` windows, C stack. Exit: 100 frozen cards ≤ 8 KB marginal each; thaw of the mail app ≤ 150 ms with identical-tree short-circuit verified; a 100,000-row list scrolls with ≤ 64 KB dirty in the renderer.

**W3 – Store, sync, `lean://` (5 weeks).** Content store, manifests, signatures, `lean://` over quinn, log and KV stores with version vectors and siblings, broker batching/wake windows/idle disconnect, reference gateway with the SSE and polling fallbacks. Exit: two devices converge the mail app's inbox log offline→online in both directions; broker holds no connection after 30 s idle (verified with `ss`); fallback ladder tests under UDP-blocked and buffering-proxy network namespaces pick levels 2 and 4 respectively.

**W4 – The web as an app, and the shell (4 weeks).** Phase-1 loader wrapped as the host-native HTML-compiler app (route = URL, `snapshot` = document mode, `update` = M6 interactive mode over deltas); shell app with cards, launcher, Just Type dispatching `intents`, deep links from `lean://` and `https://` URLs; forced colours, zoom, selection/copy across cards; TinyGo evaluation. Exit: the phase-1 corpus renders through the app path with unchanged SSIM and memory gates; Just Type "compose hello" opens the mail app at `/compose`; TinyGo hello-world measured and promoted or deferred by the 4 MB rule.

**W5 – Stack registry, supply chain, snapshots (3 weeks).** Registry with signed Merkle log; host-side promoted-stack list; snapshot generation at install; gateway serves snapshots as HTML. Lui v1 frozen (compatibility from here). Exit: a stack update not in the log is refused; snapshot HTML of the blog example validates and is indexable; Lui v0 apps rebuild to v1 with `luic --migrate`.

**W6 – Service bus and shared data (4 weeks).** `bus` provide/use with grants; Synergy-style shared services (`contacts.read`, `calendar.read`) as apps; `shared:<group>` channel scope; text CRDT as a stack library. Exit: a contacts app provides `contacts.read`; mail autocompletes from it with the grant and fails closed without; two users edit one note through the CRDT library with ≤ 2x payload metadata.

**W7 – Hardening and portability (ongoing).** Fuzz targets for the wire-form parser, delta applier, manifest parser, QUIC framing; AOT experiment per R3; second OS target for the app host.

## 9. Risks and open questions

- **R1: Lui regrows into CSS.** The mechanisms: the property list has a hard count (27); adding a property requires removing one or a written memory/network justification for why the feature cannot be a component; no property may ever become inherited; no selector syntax may ever be added; every W-milestone reviews the `DECISIONS.md` diff. Experiment: at W4, port three phase-1 corpus pages by hand to Lui and list every place a property was missed; accept only those that block layout, not those that block fidelity.
- **R2: Component-model support in wasmi lags.** Resolution: W1 ships a host-side canonical-ABI shim over core modules for the small `lean:app` world (strings, lists, records only; no resources); migrate to native support when available. If the shim exceeds 2,000 lines, reconsider WAMR behind a feature flag.
- **R3: Interpretation is too slow for the HTML-compiler app or real apps.** The HTML compiler stays native (it runs in the phase-1 sandbox), so this only concerns wasm apps. Experiment at W4: fuel-per-event histogram across the sample apps; if p95 `update`+`render` exceeds 200 ms, evaluate AOT (file-backed native code) before any JIT.
- **R4: `save-state` is wrong or expensive in real apps.** The SDK default serialises the model; the risk is models holding large caches. Mitigation: the 256 KB cap traps at freeze time in debug builds, and the SDK's `Model` derive marks fields `#[transient]`. Experiment in W2 with a deliberately heavy model.
- **R5: QUIC is blocked more often than expected on mobile networks.** The ladder handles it, but SSE through corporate proxies is unreliable. Measure in W3 across three real networks; if level 2 fails >30% of the time, promote WebSocket above SSE.
- **R6: Host diffing of full trees costs CPU per event.** Diff is keyed and linear; a 10,000-node tree should diff in a few ms. Measure in W1; if not, the SDK diffs in-guest against a retained wire-form (costs guest memory) and sends `patch` only.
- **R7: Multi-user shared data needs identity.** Deferred to W6; the open question is whether identity is a host concept (a key pair per user, bound to origins) or an app concept. Proposed: host key pair, origin-scoped derived identities, no global identity.
- **R8: "Everything is an app" needs privileged apps, which recreates a browser-chrome/content split.** Accepted; the shell and the HTML compiler are the only two, and their privileges are enumerated. Any third is a design smell to review.
- **Open: wasmi compiled-module memory.** If compiled modules are dirty and large, per-host baseline rises with every installed app. Experiment in W1: measure a 1 MB module's compiled footprint; if >2x, lazy compile per function or serialise compiled modules to disk.
- **Open: the content store and the phase-1 "no cache" stance.** Decided above (§5.1) with the privacy mitigations; revisit if an app can observe cache presence.

## 10. Prior art

- **webOS**: cards, Synergy, Just Type, the service bus (Luna), and "too many cards" memory pressure. We keep the model, fix the pressure by making frozen cards files.
- **Flash / Java applets**: per-app runtimes inside the page were the memory and security failure. We invert it: one runtime, apps as data plus logic.
- **Flutter web / CanvasKit**: proves a non-DOM UI toolkit can work on the web and proves the cost: every app ships the renderer (megabytes) and reimplements text, selection, a11y. Lui is Flutter's layout protocol with Flutter's renderer moved into the platform.
- **Gemini**: shows that a deliberately small, non-extensible format stays small; its refusal to grow is a feature. Lui borrows the attitude, not the austerity.
- **Plan 9 / X11**: the display as a server that clients describe to. X's mistake was a protocol at the pixel/primitive level, so toolkits grew above it; Plan 9's `draw` and `rio` kept the client side thin. We describe at the widget level, which is where sharing happens.
- **HyperCard**: cards as the unit of software; anyone could make one. The lesson for stacks: the authoring tool matters as much as the runtime.
- **Elm / TEA**: pure `update`/`view`, no retained widgets, trivially serialisable state. It is the card lifecycle's enabling assumption.
- **Datastar / HTMX**: the server (or here, the app) sends declarative fragments and the client patches; state lives in one place. Our delta ops are the same idea with a binary format.
- **Local-first / Automerge**: the goals (offline, multi-device, no server authority) and the warning (CRDT metadata is heavy). We take version vectors and siblings into the host and leave CRDTs to libraries.
- **Fermyon Spin**: component-model apps with capability-scoped host interfaces, composed at deploy time; the closest existing shape for `lean:app`.
- **WASI 0.2**: WIT, worlds, resources, and `wasm-tools compose`. We use the tooling and the world concept and deliberately import almost none of the standard interfaces.

### Critical Files for Implementation

- `lean-browser/crates/page-format/src/page.rs` — the `Page`/`Node` tables that gain `handlers`, `keys`, `scrolls`, `lists`, `inputs`, `app` and the Lui `NodeKind` range (§2.9).
- `lean-browser/crates/page-format/src/validate.rs` — the semantic pass must validate the new tables and the delta file.
- `lean-browser/crates/css-subset/src/style.rs` — `ComputedStyle` gains `Anchored`, `lines`, `font_step`; the Lui 27-property mapping lands here.
- `lean-browser/crates/renderer/src/main.rs` (and the layout/paint modules phase 1 plans) — multi-card mappings, inner scroll containers, keyed input state, selection, the event path to the app host.
- `lean-browser/crates/loader/src/main.rs` — becomes the host-native HTML-compiler app speaking `lean:app` (W4).
- New crates to add under `lean-browser/crates/`: `lui` (spec, KDL compiler, wire form), `wit/lean-app.wit`, `app-host` (wasmi, lifecycle), `sync-broker` (quinn, ladder, store), `stack-rust`.