# Distribution

Release packaging is described in Packages. This document covers
the notices, source material and review records that accompany distribution.

## License records

The application host uses GPL-3.0-only. Third-party components retain their own
notices and grants. Keep [LICENSE](../LICENSE), [NOTICE](../NOTICE.md),
[THIRD_PARTY](../THIRD_PARTY.md), [modification notices](../MODIFICATIONS.md)
and the [license policy](../LICENSE-POLICY.md) with the distribution.

The [review register](licensing/review-register.json) records unresolved items
for engine grant/version interpretation, presentation and contributor/contract
rights. It also records the scope of binary-source review. Package manifests
preserve this status as `legal_clearance: false`. Technical verification checks
files and notices; it does not resolve those items.

## Source material

The source archive contains application code, the engine, original font sources,
generators, build scripts, tests and documentation. Dependency notices retain
their original nested paths. The source-offline archive additionally contains
locked registry packages and a relative Cargo vendor configuration.

Distribute the matching source-offline archive alongside portable binaries and
retain their SHA-256 linkage. Assess Corresponding Source, runtime/toolchain
obligations, interactive notices and Installation Information for the actual
distribution arrangement under the applicable license terms.

Game IWADs are downloaded separately. The supplied screenshots use Freedoom;
retain their [image notices](images/LICENSE.md) and original credits when reusing
them. Firmware, simulator programs and flight databases are not included.

## Verification

```powershell
.\scripts\validate-docs.ps1
.\scripts\validate-licenses.ps1
.\scripts\prepare-release.ps1 -RequireClean
```

The validators compare the locked Windows graph with reviewed package records,
registry checksums, license texts, native-file exceptions, engine and font
provenance. Archive verification checks every recorded file and dependency
notice. Changed records require review before updating their expected hashes.

A release review also covers the real owner/contact, contributor authority,
branding, applicable agreements and Git history. Source ZIP checks do not scan
deleted Git content. The limited secret-pattern scan supplements archive review.

[Verification details](licensing/verification.md) ·
[License overview](licensing/README.md)
