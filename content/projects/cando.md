+++
title = "Cando"
template = "page.html"
weight = 2
date = 2026-04-05
[extra]
desc = "Task-oriented AI agent - spawned for a task, does the work, exits"
keywords = "cando, AI agent, task automation, software development, LLM, Gemma, Crystal"
discussion_number = 54
discussion_url = "https://github.com/orgs/developmeh/discussions/54"
+++

**[View Interactive Site →](/cando/)**

A task-oriented LLM agent written in Crystal. Not a chatbot - runs as a process, completes the task, exits.

## Design Philosophy

- **Task-oriented** - spawned with a goal, works until done
- **Tool mastery** - bash execution, file reading with structural awareness
- **Local-first** - runs Gemma 4 via Ollama, no cloud dependencies
- **Tight loop** - executes tools, feeds results back, iterates

## Technical Stack

- **Crystal** - compiled performance, Ruby-like syntax
- **Gemma 4 26B** - local model with native tool calling
- **Ollama** - model serving and inference
- **OpenTelemetry** - observability built-in

## Links

- [Interactive Site](/cando/)
- [Architecture](/cando/architecture.html)
- [Experiments](/cando/experiments.html)
- [Source Code](https://git.sr.ht/~ninjapanzer/cando)
