---
hide:
  - navigation
  - toc
---

<div class="hero" markdown>

![am logo](assets/am-500x451.png){ .hero__logo }

# Run multiple agents without the chaos.

<p class="hero__subtitle">
  <code>am</code> isolates each agent in its own branch, terminal, and container. Start solo—scale to teams—all with one command.
</p>

<div class="hero__install">
  <span class="prompt">$</span> curl -fsSL https://raw.githubusercontent.com/dstanek/agent-manager/main/install.sh | sh
</div>

<div class="hero__actions">
  <a href="getting-started/quick-start/" class="md-button md-button--primary">Get Started →</a>
  <a href="https://github.com/dstanek/agent-manager" class="md-button">View on GitHub</a>
</div>

</div>

<div class="use-cases" markdown>

## Interactive: Parallel Workstreams

Start Claude Code on a feature. Start Copilot on tests. Start another on refactoring. Each agent runs in complete isolation — its own branch, its own container, entirely unaware of the others. You direct each one individually through its chat interface.

```
am start feature-api --agent claude
am start feature-tests --agent copilot
am start feature-docs --agent claude
```

Merge the best work when you're done. No conflicts. No coordination overhead.

## Autonomous *(coming soon — `--auto`)*

Hand the agent a goal and step back. In autonomous mode the agent works independently without waiting for your input. Come back to a finished result.

```
am start big-refactor --agent claude --auto
```

## Team Orchestration *(coming soon — `--team`)*

One goal, multiple agents. `--team` launches and coordinates a team of agents working in parallel — each completely isolated, each on its own branch.

```
am start big-feature --agent claude --team
```

Merge the best work when you're done. The command is the same today as it will be tomorrow: `am start <slug> --agent <name>`. Isolation is built-in by default.

</div>

<div class="feature-grid" markdown>

<div class="feature-card" markdown>
<span class="feature-card__icon">:material-lightning-bolt:</span>

### One Command, Full Setup

`am start` creates an isolated branch, tmux window, container, and launches your agent. Everything ready to go—no manual setup.

</div>

<div class="feature-card" markdown>
<span class="feature-card__icon">:material-git:</span>

### No Conflicts, Ever

Each session gets its own `am/<slug>` branch via git worktrees or jj workspaces. Multiple agents can work in parallel—their code never interferes.

</div>

<div class="feature-card" markdown>
<span class="feature-card__icon">:material-console:</span>

### Side-by-Side Terminals

Agent on one side of a split-pane tmux window, your shell on the other. Watch multiple agents work in parallel. No context-switching.

</div>

<div class="feature-card" markdown>
<span class="feature-card__icon">:material-delete-forever:</span>

### One Command Teardown

`am destroy <slug>` stops the container, kills the window, removes the branch. Completely gone in seconds. No orphaned resources.

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
    <span class="ok">✓</span> <span class="out">Initialized in ~/myproject/.am</span>
    <span class="spacer"></span>
    <span class="prompt">$ </span><span class="cmd">am start feature --agent claude</span><br>
    <span class="ok">✓</span> <span class="out">Branch am/feature created</span><br>
    <span class="ok">✓</span> <span class="out">Session feature ready (tmux window am-feature)</span><br>
    <span class="ok">✓</span> <span class="out">Claude Code launched</span>
    <span class="spacer"></span>
    <span class="prompt">$ </span><span class="cmd">am start tests --agent copilot</span><br>
    <span class="ok">✓</span> <span class="out">Branch am/tests created</span><br>
    <span class="ok">✓</span> <span class="out">Session tests ready (tmux window am-tests)</span><br>
    <span class="ok">✓</span> <span class="out">GitHub Copilot launched</span>
    <span class="spacer"></span>
    <span class="prompt">$ </span><span class="cmd">am list</span><br>
    <span class="out">SLUG      AGENT     WINDOW        CREATED</span><br>
    <span class="out">feature   claude    am-feature    3 min ago</span><br>
    <span class="out">tests     copilot   am-tests      1 min ago</span>
    <span class="spacer"></span>
    <span class="prompt">$ </span><span class="cmd">am destroy feature --force</span><br>
    <span class="ok">✓</span> <span class="out">Container stopped · Worktree removed · Session destroyed</span>
  </div>
</div>

<div class="cta-section" markdown>

## Ready to run your first agent session?

Install `am` and get started in under five minutes.

<a href="getting-started/quick-start/" class="md-button md-button--primary">Get Started →</a>

</div>
