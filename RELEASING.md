# Releasing

This repository uses `release-plz` to automate version bumps, changelog updates, tags, GitHub releases, and crates.io publishing. GitHub Actions also builds and attaches runnable release artifacts for `putty-rs` and `putty_rs_web` on Linux, macOS, and Windows.

## Managed crates

The automated release workflow currently manages:

- `putty_core`
- `putty-rs`

## Repository setup

For automated publishing, the repository must have a crates.io token stored as the GitHub Actions secret `CARGO_REGISTRY_TOKEN`.

In GitHub, repository secrets are managed in:

- `Settings -> Secrets and variables -> Actions`

GitHub Actions also needs:

- `Workflow permissions` set to `Read and write permissions`
- `Allow GitHub Actions to create and approve pull requests` enabled

These settings are available in:

- `Settings -> Actions -> General`

## Automated release workflow

- Normal changes are merged into `main`.
- The `Release-plz PR` workflow opens or updates a release PR with version bumps and changelog updates.
- When that release PR is merged, the `Release-plz release` workflow publishes unreleased crates from `main`.

Short timeline:

1. A normal feature PR is merged into `main`.
2. `release-plz release-pr` opens or updates the release PR.
3. Nothing is published yet.
4. When the release PR is merged into `main`, `release-plz release` runs on that new `main` commit.
5. For each managed package that is ready to release, `release-plz`:
   - creates a git tag named `<package>-v<version>`
   - publishes the crate to crates.io
   - creates a GitHub Release for that tag
6. The `release-artifacts` workflow listens for the published GitHub Release and attaches binary archives for `putty-rs` releases.

In practice, the workflow is triggered by pushes to `main`:

1. A normal PR is merged into `main`.
2. That push triggers `release-plz`, which opens or updates the release PR.
3. The release PR goes through the normal `main` branch protection rules and CI checks.
4. After the release PR is merged, the resulting push to `main` triggers `release-plz` again.
5. That second run performs the actual publish step.

If the release PR is not merged immediately and more feature PRs are merged into `main`, `release-plz` updates the existing release PR instead of creating a separate permanent release branch for each merge.

The release automation is configured in [release-plz.toml](release-plz.toml) and the GitHub workflow in [`.github/workflows/release-plz.yml`](.github/workflows/release-plz.yml).

This is compatible with normal `main` branch protection, including:

- requiring pull requests before merge
- requiring status checks to pass
- blocking force pushes
- restricting branch deletion

## Manual release workflow

Manual publishing is useful for an initial release, recovery, or temporary fallback if automation is disabled.

Authenticate locally with crates.io:

```bash
cargo login
```

or provide the token via environment variable:

```bash
export CARGO_REGISTRY_TOKEN=...
```

Check packaging first:

```bash
cargo package -p putty_core
cargo package -p putty-rs
```

Run dry-runs:

```bash
cargo publish -p putty_core --dry-run
cargo publish -p putty-rs --dry-run
```

Publish in dependency order:

```bash
cargo publish -p putty_core
cargo publish -p putty-rs
```

`putty_core` must be published before `putty-rs`, because `putty-rs` depends on it as a normal crates.io dependency.

## Why `semver_check = false` for `putty-rs`

`release-plz` uses `cargo-semver-checks` to detect API-breaking changes in library crates. `putty-rs` is a binary crate, so API semver checks do not apply in a meaningful way there. `putty_core` remains the publishable library where semver compatibility matters.

## Version bump rules

`release-plz` primarily determines version bumps from commit messages.

In practice, this means the commits that are merged into `main` should follow the expected format. The PR title itself is not the main source for version bump decisions.

Recommended commit style:

- `fix: ...` for bug fixes
- `feat: ...` for new functionality
- `feat!: ...` or `refactor!: ...` for breaking changes

Typical effect:

- `fix:` -> patch bump
- `feat:` -> minor bump
- `type!:` -> major bump

For publishable library crates such as `putty_core`, semver checks can also detect API-breaking changes and influence the suggested release.

When a PR contains multiple commits, `release-plz` sees the commits that land on `main`:

- if the PR is squash-merged, the single squashed commit message is what matters most
- if the PR is merged without squash, the individual commit messages matter

Because of that, squash-merging with a clean Conventional Commit message is the simplest and most predictable workflow.

If the generated release PR proposes a version that does not fit the intended release, adjust it in the release PR before merging.

## Workflow files

GitHub Actions does not have a single central CI file that includes all other workflow files by default. Each YAML file in `.github/workflows/` is an independent workflow.

In this repository:

- [`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs build, lint, and test jobs
- [`.github/workflows/release-plz.yml`](.github/workflows/release-plz.yml) handles release PR creation and publishing
- [`.github/workflows/release-artifacts.yml`](.github/workflows/release-artifacts.yml) attaches cross-platform binary archives to `putty-rs` GitHub releases

GitHub Actions can reuse logic with reusable workflows (`workflow_call`) or composite actions, but that is optional. Separate workflow files are the simplest setup here.
