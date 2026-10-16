+++
title = "burl"
template = "page.html"
weight = 5
date = 2026-10-16
[extra]
desc = "A desktop notebook for Linux and macOS that keeps a tree of notes in one file"
keywords = "burl, notebook, note-taking, outliner, merkle tree, merge, Crystal, SQLite, myBase"

[taxonomies]
topics = ["Software Architecture"]
discussion_number = 66
discussion_url = "https://github.com/orgs/developmeh/discussions/66"
+++

burl is a desktop notebook for Linux and macOS. It keeps a tree of notes in a single `.burl` file, and every note is both a folder that can hold other notes and a document with its own text. Notes link to each other, carry attachments, hold tasks, and export to HTML.

Copies of a notebook merge back together line by line, without conflicts, because the file is a merkle tree. Two interactive pages explain how:

- **[Merging copies without conflicts, as a working model](/merkle-merge-explainer.html)**: the general model, with hashes computed live from text you edit.
- **[How burl stores a notebook](/how-burl-stores-a-notebook.html)**: the exact bytes burl hashes, the saved states it keeps, and how to check a hash against your own file.

## Links

- [Source code and install instructions](https://git.sr.ht/~ninjapanzer/burl)
- [User guide](https://git.sr.ht/~ninjapanzer/burl/tree/master/item/docs/guide/README.md)
