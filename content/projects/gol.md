+++
title = "Distributed Game of Life"
template = "page.html"
updated = 2024-09-10
+++

## Go Channel Based PoC
[GOGol Channels](https://github.com/ninjapanzer/gogol_channels)

Some work resued from [GOGol](https://github.com/ninjapanzer/gogol)

## GoL
I have always found simulations exciting. While the Game of Life is a shallow simulation it is fun how fast you can stand it up. In the old days I would always standup a language and create a Rock Paper Scissors game to prove some minor competency. Now its GoL, I like having to build an animation or a statistics engine. What has gotten to me these days is the scale of GoL and then injecting new rules.

### Distributed
So one of the things about distribution I am excited about is the noise from eventual consistency. In a traditional GoL we have what I refer to as the __World__ nothing more than a matrix of state that is roughly binary, alive or dead.

The world is pre-populated with a seed, some intentional or random spattering of alive to get the whole thing started.

Skipping the rules now we extend each cell in our matrix from a binary to a stateful object. Maybe they have names now like "bert" and "harry". They can have progeny and a history.

### Time series and geneology
My first thought was I could track this history using a time-series db, and even attempted to build on in ERLang. But then I realized I could probably make that a little more interesting if I did it with something distributed like NATS or Kafka.

## Phase 1
So the first phase here is to introduce only a distributed __World__ that can be queried from a compacted topic. Creating some form of client SDK to observe the world graph

## Phase 2
Unbound the graph, focusing only on neighborhoods and seeing if I can just query within a contiguous window of the simulation so I could have different views of the same simulation running at the same time.

## Phase 3
Heredity, try and see if I can trace the lineage of a cell through this process of events and a graph datastore to add new rules to the game as a cells heredity expands.

## DevLog

### 20 01 2024
#### Stats

Added channel based stats that help display how many broadcasts and the aggregate of life in the game.

__TODO__ Provide some change / second stats related to rendering.

The next phase is to build a more rational renderer in SDL. Ncurses is fine when doing simple displays or text overlays. But high fidelity renders require some ASICS and even using garbage built in graphics hardware acceleration will provide a better experience, and it will be a little fun.

### 19 01 2025
#### Profiling

What I have learned is that for the best effect in rendering we need to set some timing standards. The listen timing for a cell should be roughly double the heartbeat rate. Although this introduces an interesting issue where we might start blocking on our buffered channel. I don't know if the buffers should be bigger and if we should process all events. I suspect draining to the end of every channel and collecting only the final state is the correct item and listening more often.

### 15 01 2025
#### Getting Started

So while this project was going to wait for [Krappy Kafka](/i-made-a-thing/recreating-kafka-blind) I got to a point where I needed to see a simulacra of the larget project. In this PoC I also added the following requirements:
- Must used concurrency
- Must not use mutexes (So its going to be channels)

The larger scale of the work stays the same. We will establish a neighborhood of automata and then allow them to communicate with each other. In this specific scenario they will broadcast themselves to their neighbors.

Each cell owns its own channel and when the neighborhood initializes cells look for their neighbors and collect their channels. So with exception of the edges each cell holds a collection of 8 other cells. When all the cell is initialized it also gets bit vector of each of its neighbors in the order the neighbors are registered.

Here is how neighbors are registered:

```go
for i := -1; i <= 1; i++ {
  for j := -1; j <= 1; j++ {
    if i == y && j == x {
      continue
    }
    if y+i < 0 {
      continue
    }
    if y+i >= height {
      continue
    }
    if x+j < 0 {
      continue
    }
    if x+j >= width {
      continue
    }

    cell.AddChannel(cells[y+i][x+j].BroadcastChan())
  }
}
```

One caveat here is that our renderer is ncurses which tends to view the world as `y,x` not `x,y` which is a nuance but just means we tend to look our our world as a 2 dimensional array with `y` being the first index.

Assuming `x` and `y` is the location of the cell looking for its neighbors we create an offset of -1 to 1 in both horizontal and vertical directions to touch all 8 of its neighbors. You can visualize it this way:

```
+-----------+-----------+-----------+
|           |           |           |
|  (-1,-1)  |  (0,-1)   |  (1,-1)   |
|           |           |           |
+-----------+-----------+-----------+
|           |           |           |
|  (-1, 0)  |  (0, 0)   |  (1, 0)   |
|           |           |           |
+-----------+-----------+-----------+
|           |           |           |
|  (-1, 1)  |  (0, 1)   |  (1, 1)   |
|           |           |           |
+-----------+-----------+-----------+
```

So at this point we don't have self-initialization which is a problem as we are initializing the world before we start the simulation. But that is something for the future.

Because a cell needs to know its starting state and the starting state of its neighbors we also accumulate the initial bit vector using __1__ for __Alive__ and 0 for __Dead__.

```go
if cells[y+i][x+j].State() {
  cell.AddNeighborState(1)
} else {
  cell.AddNeighborState(0)
}
```

As cells live they listen for updates on its neighbors channels. These are buffered so they do not immediately block and each time a cell checks the mail it doesn't expect that a neighbor has sent them anything. So we assume no news is good news, and future state management is done by collecting a bit mask of the updates. We then compute the neighbors state by applying the bit mask repeatedly to the starting state. Ultimately, a cell is always hoping its memory is good enough to stay alive.

```go
for _, neighborChan := range c.neighborChans {
  select {
  case _ = <-neighborChan: // If a message is received on this channel
    latestStates <<= 1
    latestStates |= 1
    //gotUpdate = true
  default:
    latestStates <<= 1
    latestStates |= 0
  }
}
```

The bit vector of the initial state is in the same order as the channels the cell is listening to. This will allow us to later collect updates as a bit mask to progressing state on each cycle of the cells life.

This is a good point to mention that we are deviating from the traditional GoL structure now because we are not binding the world state and the render state of the world. Each cell has some randomness in how fast it lives and this dictates how often it produces a heartbeat and its render speed. Due to this variety of signals we tend not to see smooth growth. Instead we see snapshots because our cells are sometimes living faster than we can observe. There is some work that can be done to tune a more consistent view although its not quite as __beautiful__.

Cells only produce an event when they are alive and the moment of death. This means that when a cell comes alive it doesn't announce itself. This provides a small delay before impacting other cells. When the cells state changes we mark the change with the renderer. In this case we treat the terminal screen object as a frame buffer. While ncurses is particularly bad for reactive programming it does expose some primitives which provide an async render loop.

Under the hood
```go
// NoutRefresh, or No Output Refresh, flags the window for redrawing but does
// not output the changes to the terminal (screen). Essentially, the output is
// buffered and a call to Update() flushes the buffer to the terminal. This
// function provides a speed increase over calling Refresh() when multiple
// windows are involved because only the final output is
// transmitted to the terminal.
func (w *Window) NoutRefresh() {
	C.wnoutrefresh(w.win)
	return
}
```

Thus we can create a render loop in our main function that only calls __Update__ on the __Screen__. As I mentioned before this means not every update is rendered and we get a smooth but not very organic rendering. There is also no clear a redraw we are progressivly updating the display and accumulating writes. If you have worked in a rendering engine before you may be aware of the render loop that broadcasts a tick to all components and then draws the accumulated changes to screen. The smoothness of such renderings is based on the rate of change in distance. Our eye does a good job of building the motion between near states but not so good when the distance is rather far.
