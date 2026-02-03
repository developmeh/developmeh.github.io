+++
title = "Rust Dancing ANSI Banana with Server-Sent Events"
template = "page.html"
weight = 0
date = 2026-02-01
updated = 2026-02-02
[extra]
desc = "A Rust implementation of the dancing banana using Server-Sent Events (SSE) instead of chunked transfer encoding"
keywords = "rust, curl, sse, server-sent events, streaming, command-line, actix-web"
enable_discussions = true
+++

**Remember that dancing Ruby banana?** 🍌

Well, I couldn't help myself. After building the [Ruby version with chunked transfer encoding](../ruby-streaming-banana-dancer), I started wondering: what if we explored the *other* way to stream data to browsers and terminals? Enter the Rust implementation using Server-Sent Events.

Yeah, I rewrote it in Rust. With SSE.

So here's the thing: when you want to stream data from a server to clients, you've got options. My Ruby version uses chunked transfer encoding—basically HTTP/1.1's way of saying "I'm sending you data in pieces, and I'll tell you when each piece ends." But there's another player in town: Server-Sent Events (SSE), which is a proper protocol built on top of chunked encoding for one-way server-to-client streaming.

Why both? Because understanding the difference matters when you're building real streaming applications. Plus, Rust's async ecosystem with Actix-Web makes SSE implementation surprisingly elegant.

The best part? It works with both curl *and* web browsers. Same endpoint, different experiences. Curl gets raw ANSI animations, browsers get properly formatted SSE streams. One server, two clients, zero compromise.

Want to see how SSE differs from plain chunked encoding? Grab the code at [sse-dancing-banana](https://git.sr.ht/~ninjapanzer/sse-dancing-banana) and follow along. Or if you just want to see a banana dance: `curl -N http://localhost:8080/live`

Bottom line: Sometimes the best way to learn a protocol is to make something completely silly with it. And what's sillier than making fruit dance in your terminal?

---

Hope your terminal's ready for some Rust-powered dancing! 🍌🦀🎵

![streaming-banana](../streaming-banana.gif)

## DevLog

<div class="devlog-entry">

### 02 02 2026
#### SSE vs Chunked Encoding: What's the Difference?

When I built the Ruby version, I used chunked transfer encoding directly. It's HTTP/1.1's mechanism for streaming—you send data in chunks, each prefixed with its size in hex, terminated by a zero-length chunk. Simple, direct, low-level.

But SSE is different. It's a *protocol* built on top of chunked encoding. Think of chunked encoding as the delivery truck, and SSE as the carefully labeled packages inside. SSE defines a specific text format for events:

```
data: <your content here>
data: <more content>

```

Each event ends with a double newline. You can have multi-line data (prefix each line with `data:`), event types, IDs for reconnection, even retry hints. It's structured, and browsers have native `EventSource` API support.

Here's how the Rust code handles both in the same endpoint:

```rust
async fn live(req: HttpRequest) -> impl Responder {
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let is_curl = user_agent.contains("curl");

    // ... speed parameter parsing ...

    let stream = stream::unfold(
        FrameStream { current: 0, interval, is_curl },
        move |mut state| async move {
            actix_web::rt::time::sleep(state.interval).await;
            if state.current >= FRAMES.len() {
                state.current = 0;
            }
            let frame = FRAMES[state.current];
            let data = state.format_frame_data(frame);
            state.current += 1;
            Some((
                Ok::<_, std::convert::Infallible>(web::Bytes::from(data)),
                state,
            ))
        },
    );

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(stream)
}
```

The magic happens in `format_frame_data`. For curl, we send raw ANSI:

```rust
fn format_frame_data(&self, frame: &str) -> String {
    if self.is_curl {
        // Chunked encoding: just send the frame with ANSI clear codes
        format!("{}{}\n\n", ANSI_CLEAR, frame)
    } else {
        // SSE: format according to the SSE protocol
        let cleaned = self.strip_ansi(frame);
        let lines: Vec<&str> = cleaned.lines().collect();
        let sse_lines: Vec<String> = lines
            .iter()
            .map(|l| format!("data: {}", l))
            .collect();
        format!("{}\n\n", sse_lines.join("\n"))
    }
}
```

See the difference? For curl, we're just sending data. For browsers, we're wrapping each line in `data:` prefixes and preserving the SSE format. The browser's `EventSource` API automatically parses this.

**Why does this matter?**

1. **Reconnection**: SSE includes automatic reconnection with `Last-Event-ID`. Chunked encoding? You're on your own.
2. **Browser Support**: `EventSource` is built-in. Chunked encoding requires manual `fetch()` streaming, which is newer and less supported.
3. **Event Types**: SSE lets you send different event types on the same stream. Chunked encoding is just bytes.
4. **Simplicity**: For server-to-client streaming, SSE handles the protocol. Chunked encoding is just the transport.

**When to use what?**

- **Chunked Encoding**: When you need low-level control, binary data, or don't care about browser niceties. Think raw terminal streaming, like the Ruby version.
- **SSE**: When you want browser compatibility, automatic reconnection, structured events, or you're building a real-time notification system.

For this project, SSE won because I wanted both curl *and* browser support without writing separate endpoints.

</div>

<div class="devlog-entry">

### 02 02 2026
#### Rust's Async Streams: The Good Parts

Coming from Ruby's Sinatra with its simple `stream` block, I expected Rust to be painful. It wasn't.

Actix-Web's streaming response is built on Rust's `Stream` trait, which is like an async iterator. You create something that implements `Stream`, and the framework handles the rest:

```rust
struct FrameStream {
    current: usize,
    interval: Duration,
    is_curl: bool,
}

impl Stream for FrameStream {
    type Item = Result<web::Bytes, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>)
        -> Poll<Option<Self::Item>>
    {
        if self.current >= FRAMES.len() {
            self.current = 0;
        }
        let frame = FRAMES[self.current];
        let data = self.format_frame_data(frame);
        self.current += 1;
        Poll::Ready(Some(Ok(web::Bytes::from(data))))
    }
}
```

But I took a shortcut. Instead of implementing `Stream` manually, I used `stream::unfold`, which is like `reduce` but for streams:

```rust
let stream = stream::unfold(
    FrameStream { current: 0, interval, is_curl },
    move |mut state| async move {
        actix_web::rt::time::sleep(state.interval).await;
        // ... produce next item ...
        Some((Ok(web::Bytes::from(data)), state))
    },
);
```

The state (`FrameStream`) gets passed into the async block, which produces the next item and returns the updated state. Rinse, repeat, stream forever. It's elegant once you get past the types.

**The Rust Tax**: You pay upfront in type signatures (`Result<web::Bytes, std::convert::Infallible>` for an infallible stream?), but you get safety and zero-cost abstractions. No runtime overhead for this streaming abstraction—it compiles down to a state machine.

**The Ruby Comparison**: In Ruby's Sinatra, I did this:

```ruby
stream(:keep_open) do |out|
  loop do
    out << render_frame
    sleep 0.1
  end
end
```

Simple, but you're managing the loop and sleep manually. Rust's `stream::unfold` encodes that pattern into the type system. More verbose, but impossible to accidentally block the runtime or leak resources.

</div>

<div class="devlog-entry">

### 01 02 2026
#### Compile-Time Frame Embedding

One detail I'm proud of: the frames are embedded at compile time using `include_str!`:

```rust
const FRAMES: [&str; 8] = [
    include_str!("../../assets/frames/frame0.txt"),
    include_str!("../../assets/frames/frame1.txt"),
    include_str!("../../assets/frames/frame2.txt"),
    include_str!("../../assets/frames/frame3.txt"),
    include_str!("../../assets/frames/frame4.txt"),
    include_str!("../../assets/frames/frame5.txt"),
    include_str!("../../assets/frames/frame6.txt"),
    include_str!("../../assets/frames/frame7.txt"),
];
```

No runtime file I/O. No error handling for missing files in production. The frames are literally part of the compiled binary, stored in the `.rodata` section. If the files don't exist at compile time, the build fails. Hard fail at compile time beats mysterious runtime errors.

In Ruby, I loaded frames at runtime:

```ruby
frames = Dir.glob("ascii_frames/*.txt").sort.map { |f| File.read(f) }
```

This works, but it's runtime overhead, potential I/O errors, and requires the filesystem to be available. For a simple animation, compile-time embedding is perfect.

**Trade-off**: Binary size increases by ~8 text files. For a banana animation, I'll take it.

</div>

<div class="devlog-entry">

### 02 01 2026
#### Nix for Rust: Less Painful Than Ruby

After fighting Nix for the Ruby version's gem dependencies, Rust was refreshing:

```nix
outputs = { self, nixpkgs, ... }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        rustc
        cargo
        rust-analyzer
      ];
    };
  };
```

That's it. Cargo handles dependencies via `Cargo.lock`, which Nix respects. No gemset.nix translation layer, no bundlerEnv complexity. Rust's deterministic builds align perfectly with Nix's philosophy.

For production, I'd add `pkgs.buildRustPackage`, but for local dev? This simple shell is all you need.

The Rust ecosystem's commitment to reproducible builds (via Cargo.lock) makes Nix integration almost trivial. Ruby's dynamic nature fights Nix at every turn. This is one of those moments where Rust's compile-time philosophy pays dividends.

</div>
