# Testing PRs Before Merging

## Setup

```bash
# Make sure gh CLI is available
export PATH="/c/Users/bro/Downloads/gh_cli/bin:$PATH"
# Or install globally: winget install GitHub.cli
```

## Workflow: Test a Single PR

```bash
# 1. Fetch and checkout the PR branch
gh pr checkout <PR_NUMBER>

# 2. Build and run
cargo check                  # Fast compile check
cargo test --locked           # Run unit tests
cargo clippy --locked --all-targets --all-features -- -D warnings  # Lint
cargo run                    # Launch app for manual testing

# 3. Go back to main when done
git checkout main
```

## Workflow: Test All PRs Together

```bash
# 1. Create a temporary branch that merges all open PRs
git checkout -b test/all-prs main

# 2. Merge each PR branch (stop if conflicts arise)
gh pr list --state open --json headRefName --jq '.[].headRefName' | while read branch; do
  echo "Merging $branch..."
  git merge "origin/$branch" --no-edit || { echo "CONFLICT in $branch — resolve manually"; break; }
done

# 3. Verify everything works together
cargo check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo run

# 4. Clean up when done
git checkout main
git branch -D test/all-prs
```

## Workflow: Merge a PR

```bash
# After testing, merge from CLI:
gh pr merge <PR_NUMBER> --merge --delete-branch

# Or squash merge (cleaner history):
gh pr merge <PR_NUMBER> --squash --delete-branch
```

## What to Test Per PR

### Bug Fixes
| PR | Branch | Manual Test |
|----|--------|-------------|
| #1 | `fix/nan-panic-sort` | Create 3+ terminals, drag/resize rapidly — no crash |
| #2 | `fix/toggle-fullscreen` | F11 or Ctrl+Shift+P → "Toggle Fullscreen" — toggles |
| #8 | `fix/selection-bugs` | See below |

**PR #8 — Selection & Placement (3 tests):**
1. Run a TUI app (vim, htop), press Ctrl+C to kill it → try selecting text → should work
2. Open any CLI, click and drag to select text → should work consistently, not "1 time on 2"
3. Spawn 4-6 terminals with Ctrl+Shift+T → should fill in a grid, not diagonally

### Performance
| PR | Branch | Manual Test |
|----|--------|-------------|
| #4 | `perf/pty-repaint-throttle` | Run `seq 1 100000` — watch CPU usage, should be lower |

### Code Cleanup / CI
| PR | Branch | Manual Test |
|----|--------|-------------|
| #3 | `chore/dead-code-cleanup` | `cargo check && cargo test --locked` — just verify build |
| #7 | `ci/cross-platform` | Push branch, check GitHub Actions runs on 3 platforms |

### Features
| PR | Branch | Manual Test |
|----|--------|-------------|
| #5 | `feat/handle-terminal-events` | In vim: yank text, verify clipboard works |
| #6 | `security/updater-checksum` | `cargo test --locked` — 3 new checksum tests pass |

## Quick Commands Reference

```bash
cargo run                    # Dev build + launch
cargo check                  # Fast type check (no binary)
cargo test --locked           # Run all unit tests
cargo fmt --check             # Check formatting
cargo clippy --locked --all-targets --all-features -- -D warnings  # Lint

gh pr list                   # List open PRs
gh pr checkout <N>           # Switch to PR branch
gh pr merge <N> --squash --delete-branch  # Merge + cleanup
gh pr close <N>              # Close without merging
```
