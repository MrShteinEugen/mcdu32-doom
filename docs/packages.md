# Release packages

Build packages locally or run the [package workflow](../.github/workflows/packages.yml).
Each run creates a separate `dist/mcdu-doom-<version>-<id>/` directory.
See [distribution](publication.md) for the accompanying license records.

## Prepare a package set

On Windows, install PowerShell 7, Python 3.11 or later, Git, Rust, MSVC Build Tools
and the Windows SDK. Run these commands from the repository root:

```powershell
cargo fetch --locked
.\scripts\validate-docs.ps1
.\scripts\validate-licenses.ps1
.\scripts\prepare-release.ps1 -RequireClean
```

`-RequireClean` rejects modified and untracked files. Without it, the manifest
records the commit and a `dirty` flag. Use the file hashes to identify the exact
contents of a local package. A tag such as `v0.1.0` must match `Cargo.toml`.

## Output files

| File | Contents |
| --- | --- |
| `mcdu-doom-<v>-source.zip` | Host and engine sources, fonts, tests, scripts, documentation, images and Windows dependency notices |
| `mcdu-doom-<v>-source-offline.zip` | The source package plus locked registry dependencies and a relative Cargo vendor configuration |
| `mcdu-doom-<v>-windows-x86_64-portable.zip` | CLI and tray executables, host sources, documentation, Freedoom setup script, notices, dependency inventory and CycloneDX SBOM |
| `mcdu-doom-<v>-windows-x86_64-symbols.zip` | PDBs for both executables |
| `RELEASE-MANIFEST.json` | Archive names, sizes, hashes, commit, version, `dirty` flag and rights-review status |
| `BUILD-INFO.json` | Tool versions, target, build command and hashes of the lockfile and offline source archive |
| `SHA256SUMS.txt` | SHA-256 checksums for the final ZIP and JSON files |

Both source archives contain `PUBLICATION-MANIFEST.json`. The Windows and
symbol archives contain `ARTIFACT-MANIFEST.json`. Each manifest records file
hashes; the Windows and symbol manifests also identify the matching offline
source ZIP by SHA-256. The Windows archive repeats that link in `BUILD-INFO.json`
and includes `DEPENDENCIES.json` and `SBOM.cdx.json`.

ZIP entries are sorted and use a fixed timestamp. This makes archiving the
same input bytes repeatable with the same ZIP implementation. It does not
establish bit-for-bit reproducibility of EXE or PDB files across toolchains.
The `staging/` directory is local and is excluded from the GitHub artifact.

## Offline sources

The offline archive includes the entire locked registry graph so Cargo can
resolve dependencies without network access. This includes packages for other
targets and original dependency fixtures, such as DLLs in `libloading` tests.
These files are stored under `dependencies/` and checked against
`.cargo-checksum.json`. The [license inventory](licensing/dependencies-windows.json)
covers the selected Windows graph. The offline build does not resolve the
[open review items](licensing/review-register.json).

Toolchains, system libraries, game data and project-generated outputs such as
`target/`, logs and local configuration are excluded from source packages.
Original dependency fixtures are preserved as described above. The package
scanner checks a limited set of secret patterns; review Git history separately.

## Run the Windows package

Install the Microsoft Visual C++ v14 x64 runtime (`VCRUNTIME140.dll`) separately.
Follow `START-HERE.md` for setup and launch. Game IWADs are downloaded separately.

## Verify the archives

```powershell
python scripts/verify-release.py .\dist\mcdu-doom-<version>-<id>
.\scripts\verify-publication.ps1 -Archive .\dist\...\mcdu-doom-<version>-source.zip
```

The verifiers inspect the archives without executing their contents. They check
hashes, required files, notices and archive paths. The release verifier also
checks the AMD64 PE headers and the CLI and GUI subsystem values. Archive tests
cover changed content, unlisted files, duplicate names, case collisions and
parent-directory traversal. Executables are unsigned; obtain expected checksums
from a trusted source.

## Rebuild from offline sources

```powershell
.\scripts\rebuild-offline.ps1 -Source .\dist\...\mcdu-doom-0.1.0-source-offline.zip
```

The script verifies and extracts the ZIP, uses a fresh `CARGO_HOME` and
`CARGO_TARGET_DIR`, and builds with networking disabled. Install the compiler
and SDK first. To build from a trusted, extracted source directory directly:

```powershell
cargo build --release --locked --offline --bins --target x86_64-pc-windows-msvc
```

## Publish a release

Review the CI results, archive contents and [distribution records](publication.md).
Keep the Windows package, matching offline source archive, symbols and checksums
together. Publish the release separately from the package workflow. If a package
requires replacement, issue a new version with new hashes. Keep the matching
sources available for binaries already distributed after CI artifacts expire.
