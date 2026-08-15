#!/usr/bin/env bats
#
# Discoverability regression suite.
#
# Asserts the machine-readable layer that answer engines depend on. Every
# assertion here corresponds to something that was broken at some point, so
# treat a failure as a real regression rather than a strict-linting nit.
#
#   zola build --output-dir ./_site
#   bats tests/discoverability.bats
#
# Requires: jq. Optional: BATS_LIB_PATH for bats-assert, not used here so the
# suite runs with a bare bats-core.

SITE="${SITE:-./_site}"

setup() {
  [ -d "$SITE" ] || {
    echo "No build at $SITE. Run: zola build --output-dir $SITE" >&2
    return 1
  }
}

# Extract the Nth application/ld+json block from a built page.
ldjson() {
  local file="$1" n="${2:-1}"
  awk '/<script type="application\/ld\+json">/{f=1;next} /<\/script>/{if(f){c++;f=0;print "---SPLIT---"}} f' "$file" \
    | awk -v n="$n" 'BEGIN{RS="---SPLIT---"} NR==n'
}

# ---------------------------------------------------------------------------
# Entity graph
# ---------------------------------------------------------------------------

@test "every page carries a JSON-LD entity graph" {
  # /kwike/ and /cando/ are hand-written static HTML under static/, not Zola
  # pages, so they legitimately have no structured data.
  local missing=0
  while IFS= read -r f; do
    case "$f" in */kwike/*|*/cando/*) continue ;; esac
    grep -q 'application/ld+json' "$f" || { echo "no JSON-LD: $f"; missing=$((missing+1)); }
  done < <(find "$SITE" -name index.html)
  [ "$missing" -eq 0 ]
}

@test "all JSON-LD parses as valid JSON" {
  local bad=0
  while IFS= read -r f; do
    local n=1
    while :; do
      local block
      block="$(ldjson "$f" "$n")"
      [ -n "${block// /}" ] || break
      echo "$block" | jq empty 2>/dev/null || { echo "invalid JSON-LD in $f block $n"; bad=$((bad+1)); }
      n=$((n+1))
    done
  done < <(find "$SITE" -name index.html)
  [ "$bad" -eq 0 ]
}

@test "URLs in JSON-LD are not HTML-escaped" {
  # Tera autoescape turns https:// into https:&#x2F;&#x2F; unless every value
  # goes through `| json_encode | safe`. Valid JSON, useless URLs.
  # Scoped to JSON-LD: browsers decode entities in href attributes, so an
  # escaped href is ugly rather than broken.
  local bad=0
  while IFS= read -r f; do
    local n=1
    while :; do
      local block; block="$(ldjson "$f" "$n")"
      [ -n "${block// /}" ] || break
      case "$block" in *'https:&#x2F;'*) echo "escaped URL in $f block $n"; bad=$((bad+1)) ;; esac
      n=$((n+1))
    done
  done < <(find "$SITE" -name index.html)
  [ "$bad" -eq 0 ]
}

@test "the author is a person, not an email address" {
  run grep -h -o '<meta name="author" content="[^"]*"' "$SITE/index.html"
  [[ "$output" != *"@"* ]]
  [[ "$output" == *"Paul Scarrone"* ]]
}

@test "Person entity has sameAs, knowsAbout and a resolvable url" {
  local g; g="$(ldjson "$SITE/about/index.html" 1)"
  local person; person="$(echo "$g" | jq '.["@graph"][] | select(.["@type"]=="Person")')"
  [ "$(echo "$person" | jq -r '.name')" = "Paul Scarrone" ]
  [ "$(echo "$person" | jq '.sameAs | length')" -ge 5 ]
  [ "$(echo "$person" | jq '.knowsAbout | length')" -ge 10 ]
  [ "$(echo "$person" | jq -r '.url')" = "https://developmeh.com/about" ]
}

@test "Person declares the AWS Community Builders membership" {
  # Third-party credentials corroborate expertise in a way self-assertion
  # cannot, so losing this from the graph is a real regression.
  local g; g="$(ldjson "$SITE/about/index.html" 1)"
  local m; m="$(echo "$g" | jq '.["@graph"][] | select(.["@type"]=="Person") | .memberOf')"
  [ "$(echo "$m" | jq -r '.["@type"]')" = "ProgramMembership" ]
  [ "$(echo "$m" | jq -r '.programName')" = "AWS Community Builders" ]
  [ "$(echo "$m" | jq -r '.hostingOrganization.name')" = "Amazon Web Services" ]
}

@test "every sameAs entry is a well-formed absolute URL" {
  local g; g="$(ldjson "$SITE/about/index.html" 1)"
  local bad
  bad="$(echo "$g" | jq -r '.["@graph"][] | select(.["@type"]=="Person") | .sameAs[]' \
        | grep -cv '^https://' || true)"
  [ "$bad" -eq 0 ]
}

@test "articles reference the Person by @id rather than inlining a name" {
  local d; d="$(ldjson "$SITE/tech-dives/bats-testing-bash-like-you-mean-it/index.html" 2)"
  [ "$(echo "$d" | jq -r '.author["@id"]')" = "https://developmeh.com/about#person" ]
}

@test "the author page resolves" {
  [ -f "$SITE/about/index.html" ]
}

# ---------------------------------------------------------------------------
# Article typing
# ---------------------------------------------------------------------------

@test "posts are typed as articles, not generic WebPage" {
  local d; d="$(ldjson "$SITE/tech-dives/bats-testing-bash-like-you-mean-it/index.html" 2)"
  local t; t="$(echo "$d" | jq -r '.["@type"]')"
  [[ "$t" == "TechArticle" || "$t" == "BlogPosting" || "$t" == "Article" ]]
}

@test "articles carry og:type article and published time" {
  run grep -o '<meta property="og:type" content="article"' \
    "$SITE/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism/index.html"
  [ -n "$output" ]
  run grep -o 'article:published_time' \
    "$SITE/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism/index.html"
  [ -n "$output" ]
}

@test "landing pages are not typed as articles" {
  run grep -o '<meta property="og:type" content="website"' "$SITE/index.html"
  [ -n "$output" ]
}

@test "pillar pages expose FAQPage schema" {
  for p in tech-dives/agentic-ai-engineering tech-dives/testing-shell-scripts; do
    run grep -c '"@type": "FAQPage"' "$SITE/$p/index.html"
    [ "$output" -ge 1 ]
  done
}

# ---------------------------------------------------------------------------
# Crawl surface
# ---------------------------------------------------------------------------

@test "llms.txt exists and names the author and expertise" {
  [ -f "$SITE/llms.txt" ]
  grep -q "Paul Scarrone" "$SITE/llms.txt"
  grep -q "Areas of expertise" "$SITE/llms.txt"
  grep -q "Start here" "$SITE/llms.txt"
}

@test "llms.txt excludes stub pages" {
  # Stubs under 150 words must not appear as recommended reading.
  run grep -c 'veilid\|protobuf-changed\|/tools/\|/tdd/' "$SITE/llms.txt"
  [ "$output" -eq 0 ]
}

@test "robots.txt explicitly allows the major AI crawlers" {
  for agent in GPTBot ClaudeBot PerplexityBot Google-Extended CCBot OAI-SearchBot; do
    grep -q "User-agent: $agent" "$SITE/robots.txt" || { echo "missing: $agent"; return 1; }
  done
}

@test "robots.txt disallows nothing" {
  run grep -E '^Disallow: */' "$SITE/robots.txt"
  [ -z "$output" ]
}

@test "sitemap is valid XML and includes topic hubs" {
  python3 -c "import xml.etree.ElementTree as ET; ET.parse('$SITE/sitemap.xml')"
  run grep -c '<loc>[^<]*/topics/' "$SITE/sitemap.xml"
  [ "$output" -ge 10 ]
}

@test "sitemap URLs are not HTML-escaped" {
  run grep -c 'https:&#x2F;' "$SITE/sitemap.xml"
  [ "$output" -eq 0 ]
}

# ---------------------------------------------------------------------------
# Taxonomy
# ---------------------------------------------------------------------------

@test "topic hubs are generated" {
  [ -d "$SITE/topics" ]
  local n; n="$(find "$SITE/topics" -mindepth 1 -maxdepth 1 -type d | wc -l)"
  [ "$n" -ge 15 ]
}

@test "every topic chip href resolves to a real hub" {
  # Regression: chips were built from the raw tag value ("Agentic AI") instead
  # of the slug, so every one 404'd.
  local bad=0
  while IFS= read -r href; do
    local path="${href#https://developmeh.com}"
    [ -d "$SITE${path%/}" ] || { echo "chip 404: $href"; bad=$((bad+1)); }
  done < <(grep -rho 'class="topic-chip" href="[^"]*"' "$SITE" --include=index.html \
           | sed 's/.*href="//; s/"//' | sort -u)
  [ "$bad" -eq 0 ]
}

@test "topic hubs declare their subject" {
  local d; d="$(ldjson "$SITE/topics/agentic-ai/index.html" 2)"
  [ "$(echo "$d" | jq -r '.["@type"]')" = "CollectionPage" ]
  [ "$(echo "$d" | jq -r '.about.name')" != "null" ]
  [ "$(echo "$d" | jq '.mainEntity.itemListElement | length')" -ge 3 ]
}

# ---------------------------------------------------------------------------
# Page structure
# ---------------------------------------------------------------------------

@test "content precedes the navigation in the DOM" {
  # The nav used to render inside <main> before <article>, so every page
  # opened with ~39 unrelated link titles.
  local f="$SITE/tech-dives/bats-testing-bash-like-you-mean-it/index.html"
  local art nav
  art="$(grep -bo '<article' "$f" | head -1 | cut -d: -f1)"
  nav="$(grep -bo '<nav' "$f" | head -1 | cut -d: -f1)"
  [ "$art" -lt "$nav" ]
}

@test "every page carries exactly one h1" {
  # Bing flagged 45 pages with no h1 (2026-08-11 scan). page.html injects one
  # from the title when the content lacks its own; more than one is a
  # regression in that logic or a content heading fighting the injection.
  local bad=0
  while IFS= read -r f; do
    case "$f" in */kwike/*|*/cando/*) continue ;; esac
    local n
    n="$(grep -o '<h1' "$f" | wc -l)"
    [ "$n" -eq 1 ] || { echo "h1 count $n: $f"; bad=$((bad+1)); }
  done < <(find "$SITE" -name index.html)
  [ "$bad" -eq 0 ]
}

@test "articles show a byline with a machine-readable date" {
  local f="$SITE/tech-dives/bats-testing-bash-like-you-mean-it/index.html"
  grep -q 'class="byline"' "$f"
  grep -q '<time datetime="' "$f"
  grep -q 'rel="author"' "$f"
}

@test "internal links in pillar pages all resolve" {
  local bad=0
  for p in tech-dives/agentic-ai-engineering tech-dives/testing-shell-scripts; do
    while IFS= read -r href; do
      [ -e "$SITE${href%/}" ] || [ -d "$SITE${href%/}" ] || { echo "broken: $p -> $href"; bad=$((bad+1)); }
    done < <(grep -o 'href="/[^"#]*"' "$SITE/$p/index.html" | sed 's/href="//; s/"//' | sort -u)
  done
  [ "$bad" -eq 0 ]
}

# ---------------------------------------------------------------------------
# Config table nesting
#
# These two guard the same class of bug: a key that silently belongs to the
# wrong TOML table. Any key placed after a [extra.<table>] header joins that
# table, so appending to config.toml's [extra] section without checking what
# precedes it detaches the key from the template reading it. Both of these
# were broken in exactly that way.
# ---------------------------------------------------------------------------

@test "topic hubs render their description from config" {
  # config.extra.topic_descriptions is looked up by term.slug. A hub with no
  # description is the thin page the table exists to prevent.
  local missing=0
  while IFS= read -r d; do
    local slug; slug="$(basename "$d")"
    grep -q 'class="topic-intro"' "$d/index.html" || { echo "no description: $slug"; missing=$((missing+1)); }
  done < <(find "$SITE/topics" -mindepth 1 -maxdepth 1 -type d)
  [ "$missing" -eq 0 ]
}

@test "bluesky handle reaches the atproto meta tag" {
  # bluesky_handle sat under [extra.topic_descriptions] for a while, which left
  # config.extra.bluesky_handle undefined and this tag absent from every page.
  run grep -c 'property="atproto:handle"' "$SITE/index.html"
  [ "$output" -ge 1 ]
}
