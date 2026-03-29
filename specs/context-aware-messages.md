# Bug: Context-Aware User Messages

Commands currently emit user-facing strings that reference tmux constructs even when the
user is not running inside tmux. This is confusing.

## Problem

For example, `am destroy` says:

> "Remove worktree and kill tmux window for 'feat'? [y/N]"

…even when `$TMUX` is not set and no tmux operations will actually occur.

All user-facing strings should reflect the actual runtime context.

## Approach

Introduce a `Messages` trait (or a pair of structs — `TmuxMessages` and `PlainMessages`)
with associated constants or methods for each user-facing string. The right implementation
is chosen once at startup based on `tmux::is_in_tmux()` and threaded through (or stored
as a module-level value) so every command automatically uses context-appropriate wording.

Avoid scattering `if is_in_tmux()` checks through individual command handlers.

## Strings to audit

Go through every user-facing `println!`, `eprintln!`, and confirmation prompt in `main.rs`
and command handlers. Identify any that mention:

- "tmux window"
- "pane"
- Container-specific language when container is not configured

Produce two variants for each: one for inside-tmux, one for outside-tmux.

## Design

- Define a `Messages` trait or enum at the top of `main.rs` (or a `messages.rs` module)
- Populate it once from `tmux::is_in_tmux()` after config load
- Pass it (or a reference) into each command function that emits user-facing text
- No `is_in_tmux()` calls inside individual command handlers

## Tests

- When `$TMUX` is not set, `am destroy <slug> --force` output does not mention "tmux window"
- When `$TMUX` is set, `am destroy <slug> --force` output includes "tmux window"
- Same coverage for the `am destroy` confirmation prompt (without `--force`)

## Implementation

1. Audit all user-facing strings in command handlers
2. Create `Messages` struct (or enum) with both variants for each string
3. Instantiate at startup based on `is_in_tmux()`
4. Thread through command functions; remove any inline `is_in_tmux()` calls used only
   for string selection

## Acceptance Criteria

- Running any `am` command outside tmux never mentions "window" or "pane" in output
- Running the same command inside tmux uses tmux-aware wording
- No `is_in_tmux()` calls in command handlers for string selection purposes
