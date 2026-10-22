# License preparation verification — 2026-10-20

This records technical checks, not an opinion that permission exists for all
material. Distribution intended by the owner is free source on GitHub; the
exact owner jurisdiction remains unspecified. No GitHub upload was performed.

## Positive checks

- `scripts/validate-licenses.ps1`: **passed** for the locked default-feature
  `x86_64-pc-windows-msvc` graph: 53 packages, 114 root/nested license notices,
  registry archive SHA-256 values matched Cargo.lock, selected features and
  license alternatives matched the checked-in review. The mandatory Unicode
  component is preserved as `MIT AND Unicode-3.0`.
- Seven original license/credit text hashes, four selected HIDAPI native/build
  files and the complete 179-file engine inventory matched their records.
- `scripts/validate-docs.ps1`: documentation links, 16 PNG hashes, engine
  provenance and four original aircraft font files passed. Final counts are
  reported by the script rather than assumed to cover every file in the project.
- `cargo package --locked --allow-dirty`: packaged the expanded source set and
  compiled it successfully in Cargo's package-verification directory. Documents
  added later in this review do not change runtime code; the final source ZIP
  is checked separately.

## Failure-path and checkout checks

Performed on isolated copies under ignored `output/`, with no panel access:

| Scenario | Observed result |
| --- | --- |
| Add source tree to a temporary Git index and check it out with `core.autocrlf=true` | License and documentation checks pass. |
| Append a test line to the copied GPL text | Validator rejects the changed license/copyright text. |
| Add an unrecorded header to the copied engine | Validator rejects the unrecorded engine file. |
| Replace a reviewed dependency's selected license with GPL-2.0-only | Validator rejects the changed inventory. |

The test mutations were confined to disposable copies and restored; project
source/license text was not changed to perform these tests. Original Freedoom
copyright/credits retain their CRLF bytes via explicit `-text` attributes; text
source otherwise uses LF, and TTF/PNG remain binary.

## Source archive inspection

The final ZIP is generated only after the license validator passes. Its manifest
has hashes for all included files, the license policy/modification record,
review-register path and unresolved identifiers. Package notices preserve nested
paths, including the four HIDAPI native license files missing from the previous
collector. Inspection checks all manifest hashes, notice hashes, required policy,
font sources and generators, and excluded runtime/sensitive file categories.
The manifest deliberately retains `legal_clearance: false` and describes the
binary Corresponding Source review as not assessed. `scripts/verify-publication.ps1`
performs the archive inspection automatically after ZIP creation and can be run
again with `-Archive` for a specific file. Manifest hashes are not digital signatures.

## Limits and remaining work

No gameplay/USB/runtime code changed during this licensing work; no hardware test
or panel restart was needed. This does not replace an audit of all dependency
source, toolchain/runtime rights, contributor authority, trademark searches,
contracts, patents or jurisdiction-specific law. It does not grant permission
for commercial IWADs or a future binary distribution. The [review register](review-register.json)
still contains unresolved items and criteria for closure, including the six
DoomGeneric files' project-level license/version interpretation.
