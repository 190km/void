# Releasing Void

## What ships

- Source code (zip + tar.gz) — auto-included by GitHub
- Windows MSI installer (`void-X.Y.Z-x86_64.msi`)

macOS (.dmg) and Linux (.deb) installers will be added in a future release.

## Before releasing

Run local checks:

```bash
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked
```

Update the version in `Cargo.toml` and commit:

```bash
# edit Cargo.toml version
git add Cargo.toml Cargo.lock
git commit -m "release: X.Y.Z"
git push
```

## Cut a release

1. Go to **Actions** → **Release** → **Run workflow** on the `main` branch
2. The workflow will:
   - Read the version from `Cargo.toml`
   - Create and push a `vX.Y.Z` tag
   - Create a draft GitHub release
   - Build the Windows binary and MSI installer
   - Upload the MSI to the release
   - Publish the release

## If something goes wrong

If a build fails, the release stays in draft. To retry:

1. Delete the draft release from GitHub
2. Delete the tag: `git push origin :refs/tags/vX.Y.Z`
3. Fix the issue and push
4. Run the workflow again

## Release notes

Categories are configured in `.github/release.yml` and depend on PR labels:
`feat`, `fix`, `docs`, `chore`, `breaking-change`, etc.
