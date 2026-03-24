# Releasing Void

## Workflow

```
feature branch → PR vers canary → merge dans canary → merge dans main → auto-release
```

### Branches

| Branche | Role |
|---------|------|
| `main` | Stable. Chaque push déclenche une release automatique si la version change |
| `canary` | Staging. On merge les PRs ici, on teste, puis on merge vers main |
| `fix/*`, `feat/*`, `chore/*` | Feature branches. PR vers canary |

### CI (à chaque push/PR sur canary et main)

- `cargo fmt --check`
- `cargo clippy` sur Windows + Linux + macOS
- `cargo test` sur Windows + Linux + macOS
- `cargo build --release` sur Windows + Linux + macOS

## Ajouter une feature / fix un bug

```bash
# 1. Créer une branche depuis canary
git checkout canary
git pull
git checkout -b fix/mon-fix

# 2. Faire les changements, commit
git add .
git commit -m "fix: description du fix"

# 3. Vérifier localement
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked

# 4. Push et créer une PR vers canary
git push -u origin fix/mon-fix
gh pr create --base canary --title "fix: description" --body "..."

# 5. Quand le CI est vert → merge la PR dans canary
```

## Release une nouvelle version

```bash
# 1. Se mettre sur canary, vérifier que tout est bon
git checkout canary
git pull
cargo test --locked

# 2. Bump la version dans Cargo.toml
# Patch (1.2.0 → 1.2.1) pour bugfixes
# Minor (1.2.0 → 1.3.0) pour nouvelles features
# Major (1.2.0 → 2.0.0) pour breaking changes
sed -i 's/version = "OLD"/version = "NEW"/' Cargo.toml
cargo check  # met à jour Cargo.lock
git add Cargo.toml Cargo.lock
git commit -m "bump version to vX.Y.Z"
git push

# 3. Attendre que le CI passe sur canary

# 4. Merge vers main → release automatique
git checkout main
git merge canary
git push
```

La release se crée toute seule avec les binaires pour toutes les plateformes.
Si le tag existe déjà, la release est skip (pas de doublon).

## Ce qui est publié

| Platform | Artifact |
|---|---|
| Windows x64 | NSIS installer (`void-X.Y.Z-x86_64-setup.exe`) |
| macOS Apple Silicon | DMG (`void-X.Y.Z-aarch64-apple-darwin-setup.dmg`) |
| macOS Intel | DMG (`void-X.Y.Z-x86_64-apple-darwin-setup.dmg`) |
| Linux x64 | `.deb` + `.tar.gz` |

Source code (zip + tar.gz) inclus automatiquement par GitHub.

## Release manuelle

Toujours possible via **Actions** → **Release** → **Run workflow** sur `main`.
Option de version override disponible.

## Si quelque chose plante

Si un build fail, la release publie quand même avec les plateformes qui ont réussi.

Pour retry :
1. Supprimer la release sur GitHub
2. Supprimer le tag : `git push origin :refs/tags/vX.Y.Z`
3. Fix le problème, push
4. Relancer le workflow ou re-merge
