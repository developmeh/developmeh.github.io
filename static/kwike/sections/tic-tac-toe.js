// Tic-Tac-Toe Section - Long-running loops: Session resumption, multi-agent turns
document.addEventListener('DOMContentLoaded', () => {
    const container = document.getElementById('tic-tac-toe-content');
    if (!container) return;

    container.innerHTML = `
        <div class="project-header">
            <span class="status-badge success">Success</span>
            <p class="project-tagline">Two AI agents playing complete games: proving agents can maintain context across event cycles.</p>
            <a href="https://git.sr.ht/~ninjapanzer/tic-tac-toe" class="repo-link" target="_blank" rel="noopener">View Repository →</a>
        </div>

        <div class="project-overview">
            <div class="what-card">
                <h3>What It Does</h3>
                <p>
                    Two Claude agents play tic-tac-toe against each other. Each move is an event.
                    The game loop continues until someone wins or the board fills.
                    Neither agent runs continuously—they're invoked per-turn, resuming
                    their session each time.
                </p>
                <div class="game-visual">
                    <div class="ttt-board">
                        <div class="ttt-cell">X</div>
                        <div class="ttt-cell">O</div>
                        <div class="ttt-cell">X</div>
                        <div class="ttt-cell"></div>
                        <div class="ttt-cell">X</div>
                        <div class="ttt-cell">O</div>
                        <div class="ttt-cell">O</div>
                        <div class="ttt-cell"></div>
                        <div class="ttt-cell">X</div>
                    </div>
                    <p class="game-caption">Agent X wins with diagonal</p>
                </div>
            </div>
        </div>

        <div class="kwike-concepts">
            <h3>Kwike Concepts Demonstrated</h3>
            <div class="concepts-grid">
                <div class="concept-card">
                    <h4>Session Resumption</h4>
                    <p>
                        Each agent maintains a session file. When invoked, they resume
                        from their last state. No re-explaining the rules every turn—
                        the agent remembers the game.
                    </p>
                    <div class="code-example">
                        <code>claude --resume .kwike/sessions/player-x.json</code>
                    </div>
                </div>
                <div class="concept-card">
                    <h4>Event Type as Game History</h4>
                    <p>
                        The event type itself encodes the conversation history.
                        Each move appends to the type: <code>tictactoe.game.o.x.o.x</code>
                        The suffix indicates whose turn is next—X listens for types ending in <code>.x</code>,
                        O listens for types ending in <code>.o</code>.
                    </p>
                    <div class="code-example">
                        <pre><code># player-x/consumer.yaml
source:
  types:
    - tictactoe.game           # X starts
    - tictactoe.game.o.x       # Move 3
    - tictactoe.game.o.x.o.x   # Move 5
    - tictactoe.game.o.x.o.x.o.x  # Move 7

# player-o/consumer.yaml
source:
  types:
    - tictactoe.game.o         # Move 2
    - tictactoe.game.o.x.o     # Move 4
    - tictactoe.game.o.x.o.x.o # Move 6</code></pre>
                    </div>
                </div>
                <div class="concept-card">
                    <h4>Type Hierarchy Turn Taking</h4>
                    <p>
                        When X makes a move, it emits an event with <code>.o</code> suffix—signaling O's turn.
                        O picks it up, plays, emits with <code>.x</code> suffix. The conversation structure
                        is encoded in the type system. No coordinator, no race conditions.
                    </p>
                </div>
            </div>
        </div>

        <div class="conversation-enforcement">
            <h3>Conversation via Event Type Hierarchy</h3>
            <p>The event type encodes the full game history. Each player's <code>lifecycle.status_mapping</code> determines what suffix to append:</p>
            <div class="uniform-example">
                <div class="uniform-col">
                    <h4>player-x/consumer.yaml (lifecycle)</h4>
                    <pre><code>lifecycle:
  emit_done: true
  status_field: next
  status_mapping:
    o: o      # If next=o, append .o
    none: done  # Game over

session:
  fresh_types:
    - tictactoe.game  # X starts fresh
  resume_types:
    - tictactoe.game.o.x      # Resume
    - tictactoe.game.o.x.o.x  # Resume</code></pre>
                </div>
                <div class="uniform-col">
                    <h4>Event Type Flow</h4>
                    <pre><code>tictactoe.game
  ↓ X plays, emits .o
tictactoe.game.o
  ↓ O plays, emits .x
tictactoe.game.o.x
  ↓ X plays, emits .o
tictactoe.game.o.x.o
  ↓ O plays, emits .x
tictactoe.game.o.x.o.x
  ↓ ... until .done</code></pre>
                </div>
            </div>
            <div class="uniform-example">
                <div class="uniform-col">
                    <h4>player-x/uniform.md (output)</h4>
                    <pre><code>## Output Contract
{
  "status": "done|blocked",
  "position": 1-9,
  "mark": "X",
  "next": "o|none",  ← determines suffix
  "game_state": "in_progress|x_wins|draw"
}</code></pre>
                </div>
                <div class="uniform-col">
                    <h4>How It Works</h4>
                    <pre><code>X returns: { "next": "o" }
  → lifecycle maps "o" → ".o"
  → emits: tictactoe.game.o

O returns: { "next": "x" }
  → lifecycle maps "x" → ".x"
  → emits: tictactoe.game.o.x

X returns: { "next": "none" }
  → lifecycle maps "none" → ".done"
  → game over</code></pre>
                </div>
            </div>
            <p class="enforcement-insight">
                <strong>The type IS the conversation.</strong> No external coordinator—the event type
                hierarchy enforces turn order. Each consumer only matches its specific patterns.
                The LLM's output contract determines the next event type.
            </p>
        </div>

        <div class="architecture-diagram">
            <h3>Type-Based Turn Taking</h3>
            <div class="loop-diagram">
                <div class="loop-step">
                    <div class="loop-box daemon">
                        <strong>kwike daemon</strong>
                        <small>routes by type pattern</small>
                    </div>
                </div>
                <div class="loop-arrows">
                    <div class="loop-branch">
                        <span class="branch-label">*.x patterns</span>
                        <span class="loop-arrow">↓</span>
                        <div class="loop-box agent-x">
                            <strong>Player X</strong>
                            <small>emits *.o</small>
                        </div>
                    </div>
                    <div class="loop-branch">
                        <span class="branch-label">*.o patterns</span>
                        <span class="loop-arrow">↓</span>
                        <div class="loop-box agent-o">
                            <strong>Player O</strong>
                            <small>emits *.x</small>
                        </div>
                    </div>
                </div>
                <div class="loop-merge">
                    <span class="loop-arrow">↔</span>
                    <p class="merge-label">Turn order encoded in event type—no coordinator</p>
                </div>
            </div>
        </div>

        <div class="lesson-box key-insight">
            <h4>Key Insight: The Type IS the Conversation</h4>
            <p>
                <strong>Turn order is encoded in the event type hierarchy.</strong>
            </p>
            <p>
                The <code>lifecycle.status_mapping</code> transforms LLM output into event type suffixes.
                Each consumer declares which type patterns it listens to. The conversation structure
                emerges from configuration—no coordinator logic, no turn management code.
            </p>
            <div class="insight-details">
                <div class="insight-point">
                    <strong>Type patterns as filters:</strong> Player X only sees <code>*.x</code> events.
                    Player O only sees <code>*.o</code> events. Wrong turn? Consumer ignores it.
                </div>
                <div class="insight-point">
                    <strong>Session continuity:</strong> <code>fresh_types</code> starts new sessions,
                    <code>resume_types</code> continues existing ones. The game context persists.
                </div>
                <div class="insight-point">
                    <strong>LLM controls the flow:</strong> The agent's <code>next</code> field determines
                    who plays next. The LLM doesn't just do work—it routes the conversation.
                </div>
            </div>
        </div>

        <div class="sample-output">
            <h3>Sample Game Output</h3>
            <div class="terminal-output">
                <pre><code>$ ./game-controller.sh
[game] Starting new game
[game] Turn 1: Player X
[kwike] Consuming event: turn.requested
[agent-x] Playing center (1,1)
[game] Turn 2: Player O
[kwike] Consuming event: turn.requested
[agent-o] Playing corner (0,0)
[game] Turn 3: Player X
[agent-x] Playing corner (2,2) - threatening diagonal
...
[game] Turn 7: Player X
[agent-x] Playing (0,2) - completing diagonal
[game] Player X wins!</code></pre>
            </div>
        </div>

        <div class="project-status">
            <h3>Status</h3>
            <p>
                <strong>Complete and demonstrative.</strong> Tic-tac-toe proved that
                kwike's session resumption works for multi-turn, multi-agent scenarios.
                The pattern—shell loop coordinating resumed agent sessions—scaled
                directly into srhtd's more complex orchestration.
            </p>
        </div>
    `;
});
