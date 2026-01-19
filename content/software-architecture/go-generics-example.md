+++
title = "Go Generics Example"
template = "page.html"
weight = 0
draft = false
date = 2025-01-22
updated = 2025-01-22
[extra]
discussion_number = 24
discussion_url = "https://github.com/orgs/developmeh/discussions/24"
+++

## Go Generics an Example
So in my recent Go Game of life (GOGol) projects I have had a personal goal to define some repeatable interfaces. A __renderer__ and __game world__ that lets me plug in new implementations of a life engine and not have to change too much else. For the renderer this is pretty straight forward. We have some common rendering primitives and we can expose them a methods.

```go
type Renderer interface {
  Beep()
  Draw(string)
  DrawAt(int, int, string)
  Dimensions() (y int, x int)
  Start()
  End()
  Refresh()
  BufferUpdate()
  Clear()
}
```

Now I'll admit some of the underlying goncurses leaked into this interface but this is a work in progress and has yet to be refined. Of course my two implementations either ignore functionality or make everything a log message. I have Mock renderer which captures events as statistics and a shell renderer which displays my game board to the world. Because I am able to express most of this interaction with primitives and commands I give it a hearty thumbs up.

Now on the other hand we have a world, and the world has to describe the life within. That life is specific to the world it lives in. Life as a generalization looks a little something like this:

```go
type Life interface {
  State() bool
  SetState(bool)
}
```

In reality any consumer of the world really only needs to be able to see a individual state or possibly mutate that state. The world could be expressed like this:

```go
type World interface {
  Cells() [][]Life
  ComputeState()
  Bootstrap()
}
```

And that works just fine if we only ever need to know about cells as a RW-able entity accessible through our world. The basic game of life would call __ComputeState()__ on the world and then iterate through the __Cells()__ two dimensional array on each render tick. A little something like this:

__Display_ is the terminal screen being written to__

Methods from goncurses
- __MovePrint__
- __Refresh__

```go
for y, row := range w.Cells() {
  for x, cell := range row {
    if cell.State() {
      w.display.Display.MovePrint(y, x, "0")
    } else {
      w.display.Display.MovePrint(y, x, "-")
    }
  }
}
w.display.Display.Refresh()
```

Because everything is synced to the main render tick we don't need to include any specialized behaviour to our cells. This is he mechanism used in __tradgol__ from https://github.com/ninjapanzer/gogol/blob/01b637beca8b1123aad77390286681883edab265/cmd/tradgol/main.go

You might notice in that project I also attempted parallelgol, it was a failure because I struggled to produce generic types for world and game such that I could have radically different implementations of those entities. Time heals all wounds and for me it was understanding how a generic in Go might differ from a Generic in Java another typed language I was familiar with.

#### Here is how I thought it should go

<table style="width:100%">
<thead>
<tr>
<th style="width:50%">
My Idea
</th>
<th>
Reality
</th>
</tr>
</thead>
<tbody>
<tr>
<td>

```go
package main

type Life interface {
  State() bool
  SetState(bool)
}

type ChannelCell struct {
  Life
  state bool
}

func (c *ChannelCell) State() bool { return false }

func (c *ChannelCell) SetState(state bool) {}

type World[T Life] interface {
  Cells() [][]T
  ComputeState()
  Bootstrap()
}

type ChannelWorld[T Life] struct {
  cells    [][]*T
  initProb float64
}

func (w *ChannelWorld[T]) ComputeState() {}

func main() {
  world := &ChannelWorld[ChannelCell]{}
  ...
}
```

__ChannelCell does not satisfy Life (method SetState has pointer receiver)__ and I was stuck, CellChannel implements the Life interface and thus should be substitutable for the _Life_ generic in _ChannelWorld_. I am wrong!
</td>
<td>

```go
package main

type Life interface {
  State() bool
  SetState(bool)
}

type ChannelCell struct {
  Life
  state bool
}

func (c *ChannelCell) State() bool { return false }

func (c *ChannelCell) SetState(state bool) {}

type World[T Life] interface {
  Cells() [][]T
  ComputeState()
  Bootstrap()
}

type ChannelWorld[T ChannelCell] struct {
  cells    [][]*ChannelCell
  initProb float64
}

func (w *ChannelWorld[T]) ComputeState() {}

func main() {
  world := &ChannelWorld[ChannelCell]{}
  ...
}

```

The nuance is small but super important in its simplicity. Because ChannelWorld is going to implement World which is generic it must provide a type constraint for T. That type constraint in itself needs to be the concretion this instance of the specific struct we will use. Here is the tricky part, the type constraints when binding a generic interface to a generic implementation is a double edged sword. I presented the error about how the implementation didn't satisfy the constraint interface __Life__.
</td>
</tr>
</tbody>
</table>

#### Another example

Lets get a little wild

<table style="width:100%">
<thead>
<tr>
<th style="width:50%">
Works and implements the interface
</th>
<th>
Works but doesn't implement the interface
</th>
</tr>
</thead>
<tbody>
<tr>
<td>

```go
package main

type Life interface {
  State() bool
  SetState(bool)
}

type ChannelCell struct {
  Life
  state bool
}

func (c *ChannelCell) State() bool { return false }

func (c *ChannelCell) SetState(state bool) {}

type World[T string] interface {
  Cells() [][]T
  ComputeState()
  Bootstrap()
}

type ChannelWorld[T string] struct {
  cells    [][]*string
  initProb float64
}

func (w *ChannelWorld[T]) Cells() [][]string {
  return nil
}

func main() {
  world := &ChannelWorld[ChannelCell]{}
  ...
}
```

Focus on __type World[T string] interface__

Here the secret is the __World[T string]__ interface is constraint accepts the __ChannelWorld[T string]__ constraint and thus we met all the conditions. This is the tricky part that kept me guessing because I expected the other side to be an error which it wasn't.
</td>
<td>

```go
package main

type Life interface {
  State() bool
  SetState(bool)
}

type ChannelCell struct {
  Life
  state bool
}

func (c *ChannelCell) State() bool { return false }

func (c *ChannelCell) SetState(state bool) {}

type World[T Life] interface {
  Cells() [][]T
  ComputeState()
  Bootstrap()
}

type ChannelWorld[T string] struct {
  cells    [][]*string
  initProb float64
}

func (w *ChannelWorld[T]) Cells() [][]string {
  return nil
}

func main() {
  world := &ChannelWorld[ChannelCell]{}
  ...
}

```
Once again we are now back to this __type World[T Life] interface __

Because interface implementation is passive in Go I expected this mismatch to be an exception or a compiler error but instead what I have is an unused generic interface and a generic struct. Of course downstream I needed something that implemented World and the rest of my code broke.
</td>
</tr>
</tbody>
</table>

Anyways, this was a big learning for me, I hope it helps.
