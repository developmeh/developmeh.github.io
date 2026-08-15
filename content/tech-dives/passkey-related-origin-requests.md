+++
title = "Related Origin Requests - We Could All Use Some Validation"
template = "page.html"
weight = 0
draft = false
date = 2026-08-13
updated = 2026-08-13
slug = "passkey-related-origin-requests"

[taxonomies]
topics = ["WebAuthn", "Developer Experience"]

[extra]
schema_type = "TechArticle"
desc = "The W3C explainer for Related Origin Requests is a good start, and implementing from prose is still hard. The five item cap counts registrable-domain labels rather than domains, and the origins array turns out to be order sensitive, so the gap is worth closing with a check that runs."
keywords = "Related Origin Requests, ROR, WebAuthn, passkeys, .well-known/webauthn, eTLD+1, public suffix list, relying party id, RP ID, cross domain passkeys, ccTLD passkeys, BAD_RELYING_PARTY_ID_NO_JSON_MATCH_HIT_LIMITS, Chromium webauthn, passkey origin validator"
sitemap_priority = "0.9"

[[extra.faq]]
q = "How many origins can a .well-known/webauthn file contain?"
a = "There is no limit on origins. The limit is five unique labels, where a label is the first component of an origin's registrable domain. Fifty vanity domains all named example, across any number of top level domains, cost one label. Five differently named brands cost five."

[[extra.faq]]
q = "What counts as a label in Related Origin Requests?"
a = "Reduce the origin's host to its registrable domain, eTLD+1, which discards any subdomains, then take the part before the first dot. So example.com, test.example.org and login.example.co.uk all produce the label example."

[[extra.faq]]
q = "Do subdomains count against the Related Origin Requests label limit?"
a = "No. Reducing to the registrable domain strips subdomains before the label is taken, so shop.example.com and login.example.com are the same label as example.com. Subdomains are free."

[[extra.faq]]
q = "Does the order of origins in .well-known/webauthn matter?"
a = "Yes. The browser walks the array in order. Once five distinct labels have been seen, an origin that would introduce a sixth is skipped entirely, including its origin comparison, so it can never match even if it is exactly the caller. Origins sharing an already counted label still match. List the origins you cannot afford to lose first."

[[extra.faq]]
q = "What does BAD_RELYING_PARTY_ID_NO_JSON_MATCH_HIT_LIMITS mean?"
a = "No listed origin matched the caller, and the five label budget was exhausted before the whole array could be considered. It is the browser saying the origin may well be in the file but it stopped looking, which distinguishes a budget problem from a genuinely missing entry."

[[extra.faq]]
q = "Why does a valid .well-known/webauthn file still fail?"
a = "Common causes beyond the label budget are the wrong Content-Type, since the browser requires application/json exactly, a body over 256KB, a response slower than the ten second timeout, and any attempt to gate the file behind authentication, because the fetch is made with cookies blocked."

[[extra.faq]]
q = "Are localhost and IP address origins allowed in .well-known/webauthn?"
a = "They are skipped. An origin with no registrable domain, which covers IP addresses, localhost, and bare public suffixes such as co.uk, produces no label and is passed over rather than matched or rejected."
discussion_number = 63
discussion_url = "https://github.com/orgs/developmeh/discussions/63"
+++

# Related Origin Requests - We Could All Use Some Validation

Writing a specification is hard. Implementing one is a different kind of hard, and the second kind gets much less sympathy than it deserves.

A spec has to describe behaviour precisely enough that independent implementers converge, in prose, without running anything. Every place the prose leaves room, an implementation makes a decision, and that decision becomes the behaviour whether or not it was the intended reading. Related Origin Requests is a good example, because the [W3C explainer](https://github.com/w3c/webauthn/wiki/Explainer:-Related-origin-requests) is genuinely good and you can still deploy a file that satisfies it and does not work.

This is what I found deploying it, and why I ended up writing a checker rather than reading more carefully.

## What are Related Origin Requests for?

A passkey is bound to a relying party ID, and that ID is a domain. This is the property that makes passkeys resistant to phishing, and it is also the property that breaks the moment a brand owns more than one domain.

If you sell in eleven countries, you have eleven country code domains. If you have acquired anything, you have its domain too. Under the original rules, a passkey registered at `example.com` is unusable at `example.co.uk`, and the user is looking at a login form asking for a credential their device holds under a different name.

[Related Origin Requests](https://passkeys.dev/docs/advanced/related-origins/) is the fix. You serve a JSON file at `https://<rp-id>/.well-known/webauthn` listing the origins allowed to use that RP ID, and the browser fetches it when the origin does not match:

```json
{
  "origins": [
    "https://example.com",
    "https://example.co.uk",
    "https://example.de"
  ]
}
```

There is a cap. Everyone reads it as five domains. **It is not five domains, and the difference decides whether this feature works for you at all.**

## What is a label?

A label is the first component of an origin's *registrable domain*.

Two steps, and the first one is the one people skip. Reduce the host to its eTLD+1 using the [public suffix list](https://publicsuffix.org/), which discards every subdomain. Then take the portion before the first dot.

| Origin host | Registrable domain | Label |
|---|---|---|
| `example.com` | `example.com` | `example` |
| `shop.example.net` | `example.net` | `example` |
| `login.example.co.uk` | `example.co.uk` | `example` |
| `example.com.au` | `example.com.au` | `example` |
| `one.thing.com` | `thing.com` | `thing` |

Every row but the last produces the same label. So a file listing five origins across five different top level domains, plus every subdomain you own, consumes **one** of your five slots.

**The cap counts brands, not domains.** That is a far more generous limit than it reads as, and it is generous in exactly the direction the feature was designed for. The ccTLD estate is the motivating case, and the ccTLD estate is nearly free.

## What actually burns the budget?

Platform-hosted origins, and this is where teams get caught.

The public suffix list has a private section, and Chromium consults it with private registries included. Hosting platforms register their domains there so each customer gets an isolated registrable domain, which is correct and desirable for cookie scoping. It also means every preview deployment is its own eTLD+1, and therefore its own label.

| Origins | Unique labels |
|---|---|
| `example.com`, `example.co.uk`, `example.de`, `shop.example.com`, `login.example.com.au` | **1** |
| five `*.vercel.app` preview URLs, plus `example.com` | **6** |

Five preview URLs and your production domain exceed the budget on their own. The same applies to `*.github.io`, `*.herokuapp.com`, and anything else in the private section. If per-branch preview environments need passkeys, they need a domain you control, with the previews as subdomains, because subdomains are free and platform slugs are not.

## Why does the order of the origins array matter?

Because the browser walks it in order and stops counting, and this is the part that produces support tickets rather than build failures.

Here is the loop, from [Chromium's implementation](https://source.chromium.org/chromium/chromium/src/+/main:content/browser/webauth/webauth_request_security_checker.cc):

```cpp
constexpr size_t kMaxLabels = 5;
bool hit_limits = false;
base::flat_set<std::string> labels_seen;
for (const base::Value& origin_str : *origins) {
  // ... derive etld_plus_1_label ...
  if (!base::Contains(labels_seen, etld_plus_1_label)) {
    if (labels_seen.size() >= kMaxLabels) {
      hit_limits = true;
      continue;
    }
    labels_seen.insert(etld_plus_1_label);
  }

  const auto origin = url::Origin::Create(url);
  if (origin.IsSameOriginWith(caller_origin)) {
    return blink::mojom::AuthenticatorStatus::SUCCESS;
  }
}
```

Read what `continue` skips. When an origin would introduce a sixth label, the loop moves on **before reaching the origin comparison**. That origin can never match, even when it is character for character the caller.

Two consequences worth writing down:

**Origins sharing an already counted label are always safe.** Once `example` is in the set, every further `example.*` origin reaches the comparison regardless of position. The budget applies to new labels only.

**A sixth brand can shadow everything after it.** Add one acquisition's domain in the middle of the array and every subsequent new-label origin becomes unreachable, in array order, with no change to the file's validity. It parses, it looks right, and one region's users cannot sign in.

So the ordering rule is simple. **Put the origins you cannot afford to lose at the top.**

Chromium reports this case distinctly, which is a real kindness. `BAD_RELYING_PARTY_ID_NO_JSON_MATCH` means the origin is not in the file. `BAD_RELYING_PARTY_ID_NO_JSON_MATCH_HIT_LIMITS` means the browser gave up before it finished looking. Those call for opposite fixes, and the second one is the one that makes a correct-looking file fail.

## What else does the browser check?

Four constraints that have nothing to do with labels, and all of which are ordinary deployment problems:

**Content type must be `application/json`.** Chromium compares the MIME type exactly and returns `BAD_RELYING_PARTY_ID_WRONG_CONTENT_TYPE` otherwise. `.well-known/webauthn` has no file extension, and a good number of static hosts serve extensionless files as `text/plain` or `application/octet-stream` by default. The file is served, it is correct, and it is rejected.

**The body is capped at 256KB** and the request times out after **ten seconds**. Neither is tight, but a `.well-known` path routed through an application server rather than the CDN can miss the second one under load.

**The fetch blocks all cookies.** The request is uncredentialed, so any attempt to put the file behind a session, a WAF rule keyed on cookies, or a bot check that expects browser state, makes it unfetchable. It has to be public and boring.

**Origins with no registrable domain are skipped.** IP addresses, `localhost`, and bare public suffixes like `co.uk` produce no label and are passed over. Listing `https://localhost:3000` for local development achieves nothing, so plan a real development domain instead.

One more, easy to lose: the file is fetched from the **RP ID's** domain, not the caller's. `https://example.co.uk` asking to use RP ID `example.com` causes a fetch of `https://example.com/.well-known/webauthn`. The file lives in one place and authorises outward.

## How do you check this before shipping?

Reading the explainer tells you the rules. It cannot tell you what your file does, because that depends on the public suffix list, which is a 15,000 line document that changes, and on the order you happened to write your origins in.

So I wrote [passkey-origin-validator](/projects/passkey-origin-validator/), a Go CLI that ports the Chromium check rather than the prose. It counts labels and it answers the specific question of whether a given caller origin resolves:

```bash
# how many labels does this file actually consume?
passkey-origin-validator count example.com

# would this caller origin be authorised?
passkey-origin-validator validate example.com --origin https://example.co.uk
```

It reports the same four statuses Chromium does, including the `HIT_LIMITS` distinction, and exits non-zero on a budget warning or a validation failure so it can sit in CI beside everything else that guards a deploy. It reads a local file too, which means the check runs against the artifact before the artifact is public.

This is [rules before models](/tech-dives/agentic-ai-engineering/) applied somewhere with no models in it at all. The question "is this origin authorised" has one correct answer, derivable from the file and the public suffix list, so it belongs in a script that runs every time rather than in a runbook that says to check carefully.

## Specifications need validators, not more careful readers

None of the above is a criticism of the explainer. It is short, it is clear, and it says the limit is five and describes how to derive a label. Everything I have written here is consistent with it.

The trouble is that two of those sentences carry more than they look like they carry. "Remove any public suffix" is a reference to a live 15,000 line list, so whether `my-app-git-main.vercel.app` costs you a slot is a fact about the list this week rather than about the rule. And a prose description of a matching procedure does not commit to what happens when the budget runs out mid-array. The implementation had to decide, it decided `continue`, and that decision is the difference between a working deployment and a region full of users who cannot sign in.

Neither of those is a defect in the writing. They are the ordinary residue of describing behaviour in prose, and every specification has some. The reason it stings more here is that Related Origin Requests is young, the failure is a login failure, and the feedback loop runs through a browser you cannot step through.

Which is an argument for validators rather than for better explainers. A specification is a shared description of intent, and the thing that makes it dependable is an executable check somebody can run against their own artifact before it is anyone's outage. Conformance suites do this for older standards. Newer ones tend to get an explainer, a reference implementation buried in a browser tree, and a period where everyone rediscovers the same edges independently.

That gap is worth closing, and it is a nice class of problem, because the check is small and the ambiguity is finite. Read the implementation once, port the loop, and the answer is available to everyone who comes after without the outage.

## Related reading

- [passkey-origin-validator](/projects/passkey-origin-validator/), the tool
- [Agentic AI Engineering](/tech-dives/agentic-ai-engineering/), on putting determinism where a rule will do
- [BATS: Testing Bash Like You Mean It](/tech-dives/bats-testing-bash-like-you-mean-it/), on gating deploys with checks that actually run

Everything on this subject: [WebAuthn](/topics/webauthn/), [Developer Experience](/topics/developer-experience/)
