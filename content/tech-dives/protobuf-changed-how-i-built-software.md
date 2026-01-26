+++
title = "Protobuf - How it changed how I built software"
template = "page.html"
weight = 0
draft = true
date = 2025-10-27
updated = 2025-10-27
+++

## IDLs and what they teach

Interface Definition Languages like Protobuf, Avro, or OAS introduce an interesting pattern to how you build dependencies. Often I see IDLs as a last thought when publishing to kafka but applied to each layer of your stack has interesting results.

- Forces client generation vs static SDKs
- Makes your stack more flexible, its easier to pick mixed languages

