# Commit Trailers

`am` encourages documenting AI involvement in commits using a consistent set of git trailers. These appear at the bottom of the commit message, separated from the body by a blank line, and follow the `Key: Value` format that git and most forges recognise as structured metadata.

## Trailer reference

Two dimensions determine which trailer to use: what the agent did (piloted code or reviewed code) and how it was involved (interactively with you, or autonomously on its own).

| Trailer | Mode | When to use |
|---|---|---|
| `Co-Piloted-By` | Interactive | You worked with the agent interactively to write or modify code |
| `Auto-Piloted-By` | Autonomous | The agent wrote or modified code autonomously (`--auto`) |
| `Co-Reviewed-By` | Interactive | You and the agent reviewed the code together interactively |
| `Auto-Reviewed-By` | Autonomous | The agent reviewed the code autonomously without your direction |

The value is always `am via <agent name>`, where the agent name is the human-readable name of the agent that was involved.

## Examples

A commit where you worked interactively with Claude Code to implement a feature:

```
feat(auth): add OAuth2 login flow

Implements the authorization code flow with PKCE. Refresh tokens are
stored encrypted at rest.

---
Co-Piloted-By: am via Claude Code
```

A commit produced by an autonomous agent session:

```
fix(config): handle missing home directory gracefully

---
Auto-Piloted-By: am via Claude Code
```

A commit where code was written interactively and then reviewed by a second agent:

```
refactor(container): simplify mount resolution logic

---
Co-Piloted-By: am via Claude Code
Co-Reviewed-By: am via GitHub Copilot
```

## Placement

Trailers must be at the very end of the commit message, after a line containing only `---`:

```
<subject line>

<optional body>

---
<trailers>
```

The `---` separator is an `am` convention — it visually distinguishes human-written content from agent attribution. Git trailer parsing does not require it, but it makes the boundary explicit when reading `git log`.
