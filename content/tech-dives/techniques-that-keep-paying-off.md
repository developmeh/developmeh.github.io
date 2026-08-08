+++
title = "Five Techniques That Keep Paying Off When You Build for LLMs"
template = "page.html"
weight = 0
draft = false
date = 2026-08-08
updated = 2026-08-08
slug = "techniques-that-keep-paying-off"

[taxonomies]
topics = ["Agentic AI", "Context Engineering", "Agent Orchestration", "Capability Security"]

[extra]
schema_type = "TechArticle"
desc = "Six months of building tools for LLMs, distilled into five techniques: embedding and vector search, structural indexing, harness engineering, hooks, and why agents are better at writing code than using tools."
keywords = "LLM engineering, embeddings, vector search, cosine similarity, HNSW, IVF, structural indexing, harness engineering, tool calling, MCP cost, hooks, agent reliability, context management, nomic-embed-text, USearch, LanceDB, ChromaDB, capability scoping, AWS"
sitemap_priority = "0.9"

[[extra.faq]]
q = "Why do embeddings help when working with a large codebase?"
a = "Passing documents and a query through the same embedding model puts semantically similar text near each other in vector space, so a search returns proximity with a confidence score rather than exact term matches. That collapses the map of meaning across a large corpus, which grep cannot do."

[[extra.faq]]
q = "What is structural indexing for LLM tools?"
a = "Building a table of contents on the fly and giving the model a paged index of headings to navigate, instead of letting it grep. The model follows facts through structure rather than reading whole documents, so a TOC that resolves to line ranges beats a full-file read that gets truncated in the wrong place."

[[extra.faq]]
q = "What is a harness in LLM tooling?"
a = "The wrapper that translates a model's structured tool-call output into an actual tool invocation and feeds the result back as a turn. Understanding it explains why compaction matters, why tool calls are expensive, and why memory is a hot topic."

[[extra.faq]]
q = "What is the difference between a skill and a hook?"
a = "Skills are prompts, distilled attention, and they suffer from drift and exhaustion. Hooks are imperative and fire reliably. If you hope something happens write a skill; if you need to be sure it happens, build it into the harness or observe it with a hook."

[[extra.faq]]
q = "Why are agents better at writing code than using tools?"
a = "A long sequence of tool calls is hidden context and procedural order the model has to hold, and LLMs struggle past three or four steps. Iterating in chat to discover a process, then having the model turn a successful run into a script it executes, is more specific semantically and far more repeatable."
discussion_number = 60
discussion_url = "https://github.com/orgs/developmeh/discussions/60"
+++

Having spent around six months building tools for LLMs it's become obvious which techniques keep paying off.
- Embedding and Vector search
- Structural Indexing (Tools)
- Harness Engineering
- Hooks
- Agents are better at writing code than using tools

These things are all fundamentals in the interactions with LLMs and while they underpin agentic platforms knowing about them demystifies how much of those systems produce very shallow moats. The clever application of some of these techniques really defines the reliability of the tools you build as an LLM is only as good as the quality of its inputs.
These four split into two categories. 1 and 2 are about how we get the right information into the LLM. 3 and 4 are about how we get artifacts out of the model, while the 5th is how we enforce reliability.

### Embedding and Vectors: Deep-ish Dive
Embedding and Vector search is the most "mathematically" complex to explain but probably the easiest to implement. But lets take a moment to talk about the math at a high level.
Training did two things at once. It arranged the token embedding table so that similar tokens are geometrically close.. And it shaped all the other weight matrices so that when token vectors flow through them and interact via attention, the final pooled vector lands in a position that reflects the meaning of the whole sentence, not just the bag of tokens it contains. The output vector is the end of that pipeline.
For every token in the sequence, the model produces three vectors by multiplying the token's current representation by three separate weight matrices:
-Q (query), K (key), and V (value). Think of them as three different "views" of the same token, each shaped by training to play a specific role.
The query is what this token is looking for. ("I'm the word 'it', who am I referring to?")
The key is what each token advertises about itself. ("I'm the word 'cat,' here's my label.")
The value is what each token will actually contribute if attended to. ("If you decide I'm relevant, here's the information you'll get from me.")
Most of this is the fundamentals of how attention works. In an embedding model the output is the average of the attention produced by the input first passing through the embedding table. This works by taking a word part represented as a number, the token, and doing an index style lookup of the vector of that token. This is all deterministic for a given model and that matters later. So a word becomes tokens, a token becomes a vector, and then the model does the QKV transformations of those token vectors in sequence to build an embedding. Each layer of the model does some math, like a dot product of the layer matrix and the vector building up information that is informed by attention. Attention then being how the tokens and their ancestors relate to the training weights.
Now there are other kinds of embedding and in the early days tools like word2vec which produces a vector from averaging the first embedding table lookup. It's not very sophisticated but there is no inference and it's cheap to run.
If you wanna have a less mathematical mental model check out [https://github.com/ninjapanzer/naive_pronounceable_password](https://github.com/ninjapanzer/naive_pronounceable_password) which is a very naive form of the same process. It encompasses the "model training" and then eventually "tokenization and transformation" but with essentially 1 layer for each. If you check this out just map a token = a letter in your mind. The QKV is then for each letter what is the most likely next letter. During training, it reads a corpus, a dictionary, and establishes "weights" the probability of what letter should follow the current. The "attention" is very simple as each letter points to just one other letter.
OK, that's the hard part out of the way. Assuming you stayed on the ride we have a array of numbers that represents the full attention of our embedding model. Lets say we are using nomic-embed-text. We can take this output and store it in a vector database like USearch, LanceDB, or ChromaDB. Which for the sake of simplicity is a regular database that happens to have tools around indexing and spatial searching for vectors. Postgres has an optimized way of searching and indexing date ranges for example. USearch has the same thing for estimating the "distance" or closeness between vectors. This may relate to comparing the angles of the vectors or the magnitude, direction and scale essentially. These algorithms might be some special combination of the two properties for your use case. For searching text you will find Cosine similarity common, while you see HNSW (Hierarchical Navigable Small World) and IVF (Inverted File Index) for the indexing.

### Embedding and Vectors: Why it matters
So assuming we have a vector index for our docs we then take our query and pass it through the same model and compare the query vector proximity to the index. I mean it makes sense if two similar sentences are passed through the same model they should produce similar vectors
Practically, a lot of your work with LLMs is getting the right data in and quickly this becomes finding the right data in a codebase or a bunch of documents you already had the LLM produce. This is essential to dealing with "knowledge drift" at scale. You may hear the noise that your vibe coded codebase will collapse in 6 months. Yes and no. If you never take any functional action to deal with its context size, yea but the same happens to you over time. That's why enterprises have confluence and Jira, a place to keep the facts. As we move to more agentic coding, the facts are both coalesced by LLMs and coding agents and the agents can create a lot of docs but it quickly gets confused and fails to update all references to a concept if the documents are too long or there are too many. Again, sound familiar to any engineers you know. Point being vector embedding gives us a way to collapse the map of meaning across a huge corpus. Instead of grepping hundreds of files and hopefully getting the same terms the embedding vector of those docs once passed through the model's attention gives us semantic similarity with a confidence score and a lookup index. Those cosine similarity algorithms are giving that proximity and confidence. This set of tokens is similar to those set of tokens because their attention resolves similarly. Kinda cool right? Might take 20 minutes to index a codebase or doc repo but every search after that is greatly enhanced. As your projects grow and as you figure out interesting ways that require context from interesting sources this understanding keeps paying back.
My advice is to play around with an embedding model, maybe build something that can index a document and try a search.

### Structural Indexing (Tools)
Think of this like embedding for dummies. A model input doesn't do well when there is more than 400 lines to read on frontier models and is pretty bad at 800. So you, being a diligent coder, keep all your docs and tests up to date but the model is having a hard time finding the facts it needs just due to how deeply the details are nested in the file, or how many similarly named tests you have. This is where structure holds a lot of meaning for the model.

Think of it like building a table of contents on the fly each request. When the LLM agent looks at a markdown file, instead of letting it grep, give it a paged n-depth index of headings to search. The TOC (Table Of Contents) gives LOC (Line Of Code) blocks to then read, or possibly a new TOC if the contents are too big, and this continues until we hit paydirt.

The point being embedding is good for some things but the model thinks in a graph and follows facts. It doesn't "read whole documents" and this is why you find yourself with missed updates because the grep was just off enough or the file read was truncated just right to miss those blocks. This plays into why harness engineering is valuable, but as a counterpoint, sometimes you don't need the complexity of embedding to get good results and here we are.

### Harness Engineering
Sounds like this one will be hairy but truth is it's mostly just how we manage tool calling. Before LLMs had useful mechanisms to communicate intent for action we didn't need anything to interpret it. That's the harness, a wrapper that is the proxy from the model's structured output for tool calling into an actual tool call. It looks a little something like this:

```json
{
  "type": "tool_use",
  "id": "toolu_01ABC...",
  "name": "Read",
  "input": {
    "file_path": "/home/paulscoder/repos/some_project/main.py",
    "limit": 50
  }
}

//And the result becomes a tool_result block:
{
  "type": "tool_result",
  "tool_use_id": "toolu_01ABC...",
  "content": "  1\timport os\n  2\timport sys\n..."
}
```

Under the hood the harness interacts with the structure which could be JSON or XML and interacts with the chat turns for you.

I think that was the part that taught me the most. Everything is a turn in the chat. You send something, that's a turn. It replies, that's a turn. It calls a tool, your harness responds with data, it's all just different turns and you can immediately see why tool calls and MCPs can add up. It's all context you don't see but it's using tokens.

This is just the tip of the iceberg. Harness engineering teaches you all the things that shine a light on the LLM's weaknesses:

- Why compaction is important
- Why tool calls can be expensive
- The value of structured input and output
- Why "memory" is a hot topic

Harnesses don't need to be full suites like OpenClaw or ClaudeCode. They can be rather simple wrappers that just maintain a specific shape of context. Considering agents, when we think about security and bash calls, something like ClaudeCode is wide open and can invoke all bash, while your harness can just allow specific commands. That's the point: we keep trying to apply defense-in-depth practices to essentially un-securable tools, when our alternative is to build tools that can only do what we allow. The problem scope changes with this perspective.

### Hooks
Finally hooks. I see a great deal of importance placed on skills and rightly so, that's the simplest interface to the model without having any real understanding of how anything works. There is real power there but skills still suffer from drift and exhaustion. I think skills, or more commonly prompts, are where everyone starts but it's important to classify them for what they are: fancy prompts, snippets of distilled attention.

On the other hand hooks are both reactive and interactive, they give skills a lifecycle. If you hope something happens write a skill; if you want to make sure it happens, build it into your harness or observe it with a hook. Because hooks are imperative in nature you can trust them to fire.

There are a lot of things you can do with prompts from simply injecting a turn with specific instruction to creating locks around processes. As an example take a look at [https://git.sr.ht/~ninjapanzer/kwike-fruit-stand](https://git.sr.ht/~ninjapanzer/kwike-fruit-stand). This uses an event-driven take to make sure docs are updated when we commit, so start here: [https://git.sr.ht/~ninjapanzer/kwike-fruit-stand/tree/master/item/.githooks/post-commit](https://git.sr.ht/~ninjapanzer/kwike-fruit-stand/tree/master/item/.githooks/post-commit). The demonstration is that we want to make sure docs are always updated when certain files change. So we gate it like this:

1. When a commit happens and the author isn't the agent, we start an agent to double check the docs align with our command definitions.
2. The agent knows what files matter and it sees if the docs need updating. Our hook also limits invocation to specific files.
3. After the docs are updated the agent invokes a script (we'll talk about this next) to commit, which double checks only target files are touched and then writes the commit for the agent.

The last step is what we will talk about next. Claude can absolutely write commits but it could also do other things, and if we know exactly what we want executed don't let the agent guess, just write the code.

If you happen to want to see a more complex example that gates the commit until the agent finishes so we don't end up with multiple pushes, give this stack a look: [https://github.com/developmeh/developmeh.github.io/tree/main/.kwike](https://github.com/developmeh/developmeh.github.io/tree/main/.kwike) from my blog source.

### Agents are better at writing code than using tools
Here we are back to the concept of turns. It's very easy for a search of a large file to take 10 tool calls, then update a couple files, and make a set of commits. That's a lot of hidden context and procedural order that could be muddled. LLMs often have a problem with maintaining complex sequences of more than 3 or 4 steps and this is where a lot of the instability with agents happens. One of my regular inspirations when building agents is to discover the process by iterating with the chat and then having it turn a successful run into a script. Then have the LLM execute just that script. It makes sense if you think about how LLMs resolve problems. The more unique or specific, semantically precise, a unique vector, the less likely it is to be confused with another similar act.

The more granular the tooling the harder a time the LLM has picking the right one sometimes. But if you provide very explicit tools with specific definition and ask specifically for that action to happen, it will pick the right tool. Cycling back, this is why understanding the whole vector embedding thing is essential and worth the time going over it. It informs how LLMs "think" about our requests and how we can steer them.

So a skill that describes a specific script and offloads the understanding of the actual tooling saves tokens and reduces misbehavior. Sometimes this is positioned as a replacement for MCPs but I think that's just SEO, because the idea doesn't need conflation with a replacement for something. Make the agent write code, and execute that code. Think specific to your solutions. The lesson is that generalities and huge skill libraries are not durable for repetition without you being there to babysit the chat. If you want the boring work to happen while you go do fun things, treat the LLM like a smart executor that is just fuzzy enough to be flexible but not self orienting.

---

## A note for cloud operators

The harness argument above has a direct consequence once agents touch cloud infrastructure, and it is worth stating explicitly.

The prevailing instinct is defense-in-depth around an agent that can already do anything: guardrails, prompt-level restrictions, a reviewer watching the output. That is trying to secure something un-securable. The alternative is the one that runs through this whole piece, which is to build tools that can only do what you allow, so the question shifts from "how do I stop it doing the wrong thing" to "what does it have the ability to do at all."

On AWS the primitives for that already exist and are older than any of this. Short-lived credentials from `sts:AssumeRole`, a session policy that intersects with the role policy so the effective permissions are the smaller of the two, and a separate role per agent task rather than one set of credentials shared across a fleet. An agent authoring and running its own tooling should get a session scoped to the one bucket, the one table, the one queue it was spawned to work on. The blast radius of a confused agent is then a property of the credential rather than a property of the prompt.

This is the same shape as capability sandboxing in [beamlet](/projects/beamlet/), where the zero-value grant is pure compute and every ability is an explicit, host-side grant enforced by runtime linkage. IAM is a coarser instrument than a WASM host function, but the principle transfers: the grant is made outside the thing being granted, and the thing being granted cannot widen it.

Worth pairing with the point about agents writing code. If the durable artifact is a script the agent produced and you reviewed, that script is also the unit you scope credentials against. You are no longer granting permissions to a conversation.
