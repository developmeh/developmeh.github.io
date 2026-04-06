// Projects Intro Section - Hero with progression visual
document.addEventListener('DOMContentLoaded', () => {
    const container = document.getElementById('projects-intro-content');
    if (!container) return;

    container.innerHTML = `
        <div class="projects-narrative">
            <p class="section-intro">
                Every orchestration system needs real-world pressure to reveal its true nature.
                What follows is the progression from naive experimentation to production-ready patterns—
                each project teaching something the previous one couldn't.
            </p>
        </div>

        <div class="project-timeline">
            <div class="timeline-track">
                <div class="timeline-line"></div>

                <div class="timeline-node" data-project="fruit-stand">
                    <div class="node-marker poc"></div>
                    <div class="node-content">
                        <span class="status-badge poc">POC</span>
                        <h4>fruit-stand</h4>
                        <p>Git hooks + auto-docs</p>
                    </div>
                </div>

                <div class="timeline-node" data-project="feature-forge">
                    <div class="node-marker failure"></div>
                    <div class="node-content">
                        <span class="status-badge failure">Failure</span>
                        <h4>feature-forge</h4>
                        <p>4-stage pipeline collapse</p>
                    </div>
                </div>

                <div class="timeline-node" data-project="tic-tac-toe">
                    <div class="node-marker success"></div>
                    <div class="node-content">
                        <span class="status-badge success">Success</span>
                        <h4>tic-tac-toe</h4>
                        <p>Long-running loops</p>
                    </div>
                </div>

                <div class="timeline-node" data-project="srhtd">
                    <div class="node-marker success"></div>
                    <div class="node-content">
                        <span class="status-badge success">Manifestation</span>
                        <h4>srhtd</h4>
                        <p>Enterprise orchestration</p>
                    </div>
                </div>
            </div>
        </div>

        <div class="journey-insight">
            <blockquote>
                "The pattern that emerged: thin CLI bookends wrapping kwike workflows.
                The shell handles coordination; the agent handles cognition."
            </blockquote>
        </div>

        <div class="projects-teaser">
            <div class="teaser-grid">
                <a href="#fruit-stand" class="teaser-card">
                    <span class="teaser-icon">🍎</span>
                    <h4>Fruit Stand</h4>
                    <p>Where it began: can an agent maintain its own documentation?</p>
                </a>
                <a href="#feature-forge" class="teaser-card">
                    <span class="teaser-icon">⚠️</span>
                    <h4>Feature Forge</h4>
                    <p>Ambition meets reality: why JSON contracts need enforcement.</p>
                </a>
                <a href="#tic-tac-toe" class="teaser-card">
                    <span class="teaser-icon">🎮</span>
                    <h4>Tic-Tac-Toe</h4>
                    <p>Two agents, one game: session resumption in action.</p>
                </a>
                <a href="#srhtd" class="teaser-card">
                    <span class="teaser-icon">🚀</span>
                    <h4>SRHTD</h4>
                    <p>The beads orchestrator reborn: sr.ht todo automation.</p>
                </a>
            </div>
        </div>
    `;

    // Add click handlers for timeline nodes
    const timelineNodes = container.querySelectorAll('.timeline-node');
    timelineNodes.forEach(node => {
        node.addEventListener('click', () => {
            const project = node.getAttribute('data-project');
            const targetSection = document.getElementById(project);
            if (targetSection) {
                targetSection.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        });
    });
});
