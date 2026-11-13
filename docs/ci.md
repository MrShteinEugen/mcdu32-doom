# Continuous integration

[ci.yml](../.github/workflows/ci.yml) runs on branch pushes, pull requests,
manual requests and calls from other workflows. The
[setup action](../.github/actions/setup/action.yml) installs the toolchains.
Use the commands in [CONTRIBUTING](../CONTRIBUTING.md) to run checks locally.

## Required checks

A new run cancels the previous run for the same branch. Matrix jobs continue
independently after a failure. The `CI gate` job requires every listed job to
succeed. Select `CI gate` as a required check in the GitHub branch protection rules.

| Job | Environment | Checks |
| --- | --- | --- |
| Workflow policy | Ubuntu 24.04 | Action pins, permissions, archive rejection tests and actionlint 1.7.12 |
| Quality | Windows 2022, Rust 1.98.1 | Formatting, Clippy for all targets and features, Rustdoc, PowerShell syntax, document and license checks, Cargo package build and source archive verification |
| Windows tests | Windows 2022 with Rust 1.85.0 and 1.98.1; Windows 2025 with Rust 1.98.1 | Ordinary tests, doctests, release binaries and CLI smoke tests |
| Freedoom | Windows 2022, Rust 1.98.1 | Verified IWAD, selected integration tests and gallery capture |
| Font reproduction | Windows 2022, Python 3.12, Pillow 12.2.0 | Rebuild the bitmap and compare it with the checked-in asset |
| Portability | Ubuntu 24.04, Rust 1.98.1 | Build checks for all targets and ordinary tests with the required HID and window libraries |

The Rust 1.85.0 job checks the minimum supported version. Linux is a portability
check; the release archives and license inventory target Windows x64/MSVC.

## Game tests without hardware

`scripts/setup-freedoom.ps1` verifies the archive and IWAD. CI runs the ignored
cases in `cli`, `engine_smoke`, `engine_ui`, `emulator_smoke`, `mcdu_service` and
`service` with `--ignored --test-threads=1`. These tests use synthetic input and
require no physical panel. `engine_ui` tests firing, menu and message output,
and viewport dimensions. The service tests exercise display modes, navigation,
settings and channel shutdown. Gallery captures go to `output/ci-gallery`.
The published images remain untouched.

## Artifacts

| Artifact | Contents | Retention |
| --- | --- | ---: |
| `source-review-and-rustdoc` | Verified source ZIP and Rustdoc | 14 days |
| `windows-evidence-<os>-<rust>` | CLI and probe results, page captures and atlas | 14 days |
| `freedoom-captures-and-metrics` | PNGs, JSON metrics, engine text and gallery | 14 days |
| `mcdu-doom-review-packages` | Four ZIPs, `SHA256SUMS.txt`, release manifest and build information | 30 days |
| `rustsec-report`, `rust-coverage` | RustSec JSON and LCOV | 30 days |

CI uploads diagnostic artifacts even when a job fails, if the files exist.
For a failed check, start with the failing step and then inspect its output.
Review changes to protocol contracts, licenses or fonts before updating hashes.
For a minimum-version failure, check the changed dependencies' `rust-version`.

## Package workflow

[packages.yml](../.github/workflows/packages.yml) runs on `v*` tags or on manual
request. It runs CI first, then:

1. Requires a clean checkout and checks the tag against the package version.
2. Creates and verifies the four archives.
3. Builds from the extracted offline source archive using an empty Cargo home
   and a separate target directory.
4. Checks the Windows CLI version and atlas export without WAD or USB access.
5. Runs the standalone archive verifier and uploads the release files.

The workflow produces artifacts. Publishing a GitHub Release is a separate
operation. See [packages](packages.md) for local preparation and verification.

## Scheduled checks

[security.yml](../.github/workflows/security.yml) runs Mondays at 05:23 UTC and
on manual request. `cargo-audit 0.22.2` checks all entries in `Cargo.lock` against
RustSec. `cargo-llvm-cov 0.9.1` collects LCOV for ordinary Rust tests; the report
has no minimum coverage threshold. Neither job measures hardware behavior or
C engine coverage. Dependabot checks Cargo and GitHub Actions weekly, with up to
five open pull requests per ecosystem.

## Permissions and caches

Actions are pinned to full commit SHAs. Workflows use `contents: read` and
checkouts do not persist credentials. The cache contains the registry index,
original `.crate` files and dependency Git data. Its key includes the OS,
toolchain and lockfile hash. Build products and extracted registry sources are
excluded. The offline rebuild uses a fresh cache.

Physical panel output, reconnection, font acceptance and interactive tray behavior
are outside the CI matrix.
