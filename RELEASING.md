# Releasing Void

## What ships

The release workflow builds and publishes:

- Linux: tarball plus shell installer
- macOS: tarball plus shell installer
- Windows: zip, PowerShell installer, and MSI

The current pipeline does not produce signed macOS app bundles or signed Windows installers. Add platform signing before broad public distribution.

## Before tagging

Run the local checks:

```bash
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked
```

Update:

- `Cargo.toml` version
- PR labels if you want clean GitHub-generated release note categories

## Cut a release

```bash
git add Cargo.toml Cargo.lock
git commit -m "release: X.Y.Z"
git tag vX.Y.Z
git push
git push --tags
```

Pushing the `v*` tag triggers `.github/workflows/release.yml`.

## What GitHub Actions does

1. Runs `dist host --steps=create` to compute the release plan.
2. Builds local artifacts for Linux, macOS, and Windows.
3. Builds global artifacts such as checksums and installers.
4. Uploads artifacts and creates the GitHub Release with GitHub-generated notes.

## Release Notes Format

The published release body uses GitHub's automatic release notes, so it can produce sections like:

- `What's Changed`
- `New Contributors`
- `Full Changelog`

Categories are configured in `.github/release.yml` and depend on PR labels. For best results:

- merge changes through pull requests
- apply labels like `feat`, `fix`, `docs`, `chore`, or `breaking-change`

## First-time GitHub setup

Make sure the repository has:

- GitHub Actions enabled
- Permission for workflows to create releases
- A public `repository` URL in `Cargo.toml`

Optional later improvements:

- Windows code signing
- macOS signing and notarization
- Homebrew, winget, `.deb`, `.rpm`, or `.dmg` publishing
