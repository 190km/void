# Releasing Void

## Workflow

```
feature branch → PR to canary → merge into canary → merge into main → auto-release
```

### Branches

| Branch | Role |
|--------|------|
| `main` | Stable. Each push triggers an automatic release if the version changed |
| `canary` | Staging. PRs are merged here, tested, then merged into main |
| `fix/*`, `feat/*`, `chore/*` | Feature branches. PR into canary |

### CI (on every push/PR to canary and main)

- `cargo fmt --check`
- `cargo clippy` on Windows + Linux + macOS
- `cargo test` on Windows + Linux + macOS
- `cargo build --release` on Windows + Linux + macOS

## Adding a feature / fixing a bug

```bash
# 1. Create a branch from canary
git checkout canary
git pull
git checkout -b fix/my-fix

# 2. Make changes, commit
git add .
git commit -m "fix: description of the fix"

# 3. Verify locally
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked

# 4. Push and create a PR to canary
git push -u origin fix/my-fix
gh pr create --base canary --title "fix: description" --body "..."

# 5. When CI is green → merge the PR into canary
```

## Releasing a new version

```bash
# 1. Switch to canary, verify everything is good
git checkout canary
git pull
cargo test --locked

# 2. Bump the version in Cargo.toml
# Patch (1.2.0 → 1.2.1) for bugfixes
# Minor (1.2.0 → 1.3.0) for new features
# Major (1.2.0 → 2.0.0) for breaking changes
sed -i 's/version = "OLD"/version = "NEW"/' Cargo.toml
cargo check  # updates Cargo.lock
git add Cargo.toml Cargo.lock
git commit -m "bump version to vX.Y.Z"
git push

# 3. Wait for CI to pass on canary

# 4. Merge into main → automatic release
git checkout main
git merge canary
git push
```

The release is created automatically with binaries for all platforms.
If the tag already exists, the release is skipped (no duplicates).

## What gets published

| Platform | Artifact |
|---|---|
| Windows x64 | NSIS installer (`void-X.Y.Z-x86_64-setup.exe`) |
| macOS Apple Silicon | DMG (`void-X.Y.Z-aarch64-apple-darwin-setup.dmg`) |
| macOS Intel | DMG (`void-X.Y.Z-x86_64-apple-darwin-setup.dmg`) |
| Linux x64 | `.deb` + `.tar.gz` |

Source code (zip + tar.gz) is auto-included by GitHub.

## Manual release

Still available via **Actions** → **Release** → **Run workflow** on `main`.
Version override option available.

## If something goes wrong

If a build fails, the release still publishes with whichever platforms succeeded.

To retry:
1. Delete the release on GitHub
2. Delete the tag: `git push origin :refs/tags/vX.Y.Z`
3. Fix the issue, push
4. Re-run the workflow or re-merge
