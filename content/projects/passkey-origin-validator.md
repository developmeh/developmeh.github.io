+++
title = "Passkey Origin Validator"
template = "page.html"
weight = 6
date = 2026-08-13
updated = 2026-08-13
slug = "passkey-origin-validator"

[taxonomies]
topics = ["WebAuthn", "Developer Experience"]

[extra]
schema_type = "TechArticle"
desc = "A Go CLI that ports Chromium's Related Origin Requests check, counting eTLD+1 labels in a .well-known/webauthn file and answering whether a given caller origin would actually be authorised."
keywords = "passkey origin validator, .well-known/webauthn, Related Origin Requests, ROR, WebAuthn, passkeys, eTLD+1, public suffix list, Chromium webauthn, Go CLI, RP ID validation"
categories = "Projects"
+++

Passkey Origin Validator is a Go CLI that checks a `.well-known/webauthn` file the way a browser does. It counts the eTLD+1 labels the file consumes, and it answers the one question that matters on the day something breaks: would this specific caller origin be authorised?

It is a port of Chromium's check rather than a reading of the spec. The [reference implementation](https://source.chromium.org/chromium/chromium/src/+/main:content/browser/webauth/webauth_request_security_checker.cc) is vendored into the repository under `chromium_reference/` so the two can be diffed when upstream moves.

## Why port instead of interpret

The [W3C explainer](https://github.com/w3c/webauthn/wiki/Explainer:-Related-origin-requests) for [Related Origin Requests](/tech-dives/passkey-related-origin-requests/) is clear and short, and a correct reading of it still does not tell you whether your file works. Two details are the reason.

The five item cap counts **labels**, not origins, where a label is the first component of the registrable domain. Working that out for any given host means resolving the public suffix list, which is a 15,000 line document that changes. And the browser's loop skips the origin comparison for any origin that would introduce a label past the budget, so **the array is order sensitive** in a way no schema check would surface.

Both are properties of an implementation. Porting the implementation is the only way to be sure.

## The label rule

| Origin host | Registrable domain | Label |
|---|---|---|
| `example.com` | `example.com` | `example` |
| `test.example.org` | `example.org` | `example` |
| `login.example.co.uk` | `example.co.uk` | `example` |
| `one.thing.com` | `thing.com` | `thing` |

Subdomains never produce a distinct label, and a name repeated across top level domains costs one. Hosts with no registrable domain, meaning IP addresses, `localhost`, and bare public suffixes, are skipped exactly as Chromium skips them.

eTLD+1 resolution uses [`golang.org/x/net/publicsuffix`](https://pkg.go.dev/golang.org/x/net/publicsuffix), which carries the same private section Chromium consults with `INCLUDE_PRIVATE_REGISTRIES`. That is what makes each `*.vercel.app` or `*.github.io` origin its own label.

## Commands

```bash
# count the labels a domain's file consumes
passkey-origin-validator count example.com

# ask whether one caller origin is authorised
passkey-origin-validator validate example.com --origin https://example.co.uk

# run either against a local file, before it is published
passkey-origin-validator count --file ./webauthn.json
```

Both commands take `--debug` for the resolved labels, and `--file` so a check can run against the build artifact rather than a deployed URL.

## Statuses

The four statuses are Chromium's, spelled the same way so a browser-side error can be pasted into a search and land somewhere useful:

| Status | Meaning |
|---|---|
| `SUCCESS` | the caller origin is authorised |
| `BAD_RELYING_PARTY_ID_JSON_PARSE_ERROR` | not valid JSON, or no `origins` array |
| `BAD_RELYING_PARTY_ID_NO_JSON_MATCH` | no listed origin matched |
| `BAD_RELYING_PARTY_ID_NO_JSON_MATCH_HIT_LIMITS` | no match, and the label budget ran out before the array did |

The last one is the whole reason the tool exists. It separates "your origin is missing" from "your origin is present and the browser stopped before reaching it," which are the same symptom and opposite fixes.

## Built for CI

Exit codes are distinct so the tool can gate a deploy rather than inform a human:

| Exit | Meaning |
|---|---|
| `0` | labels within the limit |
| `1` | could not fetch or parse the endpoint |
| `2` | label count exceeds the limit |
| `3` | caller origin is not authorised |

The fetch mirrors the browser's constraints, a 256KB body cap, a ten second timeout, and a required `application/json` content type, so a file that passes locally and fails in CI is telling you something true about how it is being served.

The validation logic lives in `internal/counter` and is usable directly from Go:

```go
status := counter.ValidateWellKnownJSON("https://example.com", jsonBytes)
if status != counter.StatusSuccess {
    // handle it
}
```

## What it does not do

It does not register or authenticate anything, it has no opinion on your authenticator, and it does not check the rest of the WebAuthn ceremony. It answers one question about one file.

It also tracks Chromium specifically. Other browsers implement Related Origin Requests, and where they diverge, this tool follows Chromium.

## Links

- [Source Code](https://github.com/developmeh/passkey-origin-validator)
- [Related Origin Requests: Five Labels, Not Five Domains](/tech-dives/passkey-related-origin-requests/), the write-up
- [Chromium WebAuthn implementation](https://source.chromium.org/chromium/chromium/src/+/main:content/browser/webauth/webauth_request_security_checker.cc), the reference this mirrors
- [Related Origin Requests explainer](https://github.com/w3c/webauthn/wiki/Explainer:-Related-origin-requests), W3C
