---
hide:
  - navigation
  - toc
---

<div class="hero" markdown>

![am logo](assets/am-500x451.png){ .hero__logo }

# One command to launch an agent. One more to launch a team.

<p class="hero__subtitle">
  From your first agent session to a fleet running in parallel — <code>am</code> keeps every agent isolated, organized, and out of each other's way.
</p>

<div class="hero__install">
  <span class="prompt">$</span> curl -fsSL https://raw.githubusercontent.com/dstanek/agent-manager/main/install.sh | sh
</div>

<div class="hero__actions">
  <a href="getting-started/quick-start/" class="md-button md-button--primary">Get Started →</a>
  <a href="https://github.com/dstanek/agent-manager" class="md-button">View on GitHub</a>
</div>

</div>

<div class="feature-grid" markdown>

<div class="feature-card" markdown>
<span class="feature-card__icon">:material-source-branch:</span>

### Isolated Branches

Every session gets its own `am/<slug>` branch via git worktrees or jj workspaces. Changes from different agents never collide.

</div>

<div class="feature-card" markdown>
<span class="feature-card__icon">:material-console:</span>

### Dedicated Terminals

A split-pane tmux window per session. Agent on one side, your shell on the other. No tab-switching.

</div>

<div class="feature-card" markdown>
<span class="feature-card__icon">:material-cube-outline:</span>

### Container Isolation

Rootless Podman or Docker wraps each agent in a hard boundary. Credentials mount in at runtime — nothing is baked into the image.

</div>

<div class="feature-card" markdown>
<span class="feature-card__icon">:material-robot:</span>

### Agent-Agnostic

Built-in integrations for Claude Code, GitHub Copilot, Gemini, Codex, and Aider handle credential mounting automatically. Any executable works as a custom agent.

</div>

</div>


<div class="terminal-demo">
  <div class="terminal-demo__bar">
    <span class="terminal-demo__dot terminal-demo__dot--red"></span>
    <span class="terminal-demo__dot terminal-demo__dot--yellow"></span>
    <span class="terminal-demo__dot terminal-demo__dot--green"></span>
    <span class="terminal-demo__title">myproject</span>
  </div>
  <div class="terminal-demo__body">
    <span class="prompt">$ </span><span class="cmd">am init</span><br>
    <span class="ok">✓</span> <span class="out">Created .am/ in ~/myproject</span>
    <span class="spacer"></span>
    <span class="prompt">$ </span><span class="cmd">am start feat --agent claude</span><br>
    <span class="ok">✓</span> <span class="out">Branch am/feat created</span><br>
    <span class="ok">✓</span> <span class="out">tmux window am-feat ready</span><br>
    <span class="ok">✓</span> <span class="out">Container am-feat starting...</span><br>
    <span class="ok">✓</span> <span class="out">Claude Code launched</span>
    <span class="spacer"></span>
    <span class="prompt">$ </span><span class="cmd">am start auth --agent claude</span><br>
    <span class="ok">✓</span> <span class="out">Branch am/auth created</span><br>
    <span class="ok">✓</span> <span class="out">tmux window am-auth ready</span><br>
    <span class="ok">✓</span> <span class="out">Container am-auth starting...</span><br>
    <span class="ok">✓</span> <span class="out">Claude Code launched</span>
    <span class="spacer"></span>
    <span class="prompt">$ </span><span class="cmd">am list</span><br>
    <span class="out">SLUG   AGENT    WINDOW     CREATED</span><br>
    <span class="out">feat   claude   am-feat    2 min ago</span><br>
    <span class="out">auth   claude   am-auth    just now</span>
    <span class="spacer"></span>
    <span class="prompt">$ </span><span class="cmd">am clean feat --force</span><br>
    <span class="ok">✓</span> <span class="out">Container am-feat stopped</span><br>
    <span class="ok">✓</span> <span class="out">Worktree .am/worktrees/feat removed</span><br>
    <span class="ok">✓</span> <span class="out">Session feat cleaned up</span>
  </div>
</div>

<div class="cta-section" markdown>

## Ready to get started?

Install `am` in seconds and run your first agent session in under five minutes.

<a href="getting-started/quick-start/" class="md-button md-button--primary">Get Started →</a>

</div>
