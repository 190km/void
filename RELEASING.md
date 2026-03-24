# Releasing Void

## What ships

| Platform | Artifact |
|---|---|
| Windows x64 | NSIS installer (`void-X.Y.Z-x86_64-setup.exe`) |
| macOS Apple Silicon | DMG (`void-X.Y.Z-aarch64-apple-darwin-setup.dmg`) |
| macOS Intel | DMG (`void-X.Y.Z-x86_64-apple-darwin-setup.dmg`) |
| Linux x64 | `.deb` + `.tar.gz` |

Source code (zip + tar.gz) is auto-included by GitHub.

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
2. Optionally provide a version override (otherwise reads from `Cargo.toml`)
3. The workflow will:
   - Resolve the version (from input or `Cargo.toml`)
   - Build for all platforms (Windows, macOS ARM64 + x64, Linux)
   - Package installers (NSIS `.exe`, `.dmg`, `.deb`, `.tar.gz`)
   - Create a GitHub release with auto-generated release notes
   - Upload all artifacts

The release publishes even if some platforms fail (as long as at least one succeeds).

## If something goes wrong

If a build fails, the release still publishes with whichever platforms succeeded. To retry a failed platform:

1. Delete the release from GitHub
2. Delete the tag: `git push origin :refs/tags/vX.Y.Z`
3. Fix the issue and push
4. Run the workflow again

## Release notes

Categories are configured in `.github/release.yml` and depend on PR labels:
`feat`, `fix`, `docs`, `chore`, `breaking-change`, etc.
