# Licensing overview

Original host contributions use GPL-3.0-only. Third-party components retain their
own notices and terms. See the [license policy](../../LICENSE-POLICY.md) for the
scope of these declarations and the [attribution record](../../THIRD_PARTY.md)
for component sources.

| Component | License or record |
| --- | --- |
| Rust/C host, scripts and original documentation | [GPL-3.0-only](../../LICENSE) |
| DOOM/Chocolate Doom engine | Individual GPL-2.0-or-later notices; see the review register for files without individual grants |
| `sha1.c` | GPL-3.0-or-later, Free Software Foundation |
| mcdu-dotnet transaction map | BSD-3-Clause, Andrew Whewell |
| Aircraft fonts and page templates | GPLv3, Tyler Knox and FlyByWire Simulations |
| Freedoom game data | BSD-3-Clause; downloaded separately |
| Rust and native dependencies | [Windows inventory](dependencies-windows.json) and [native component records](native-components.json) |

## Review status

The [review register](review-register.json) records the evidence and closure
criteria for `DG-VERSION` (engine license scope), `PRESENTATION` (branding and
visuals), `OWNER-CONTRACTS` (authority and agreements), and `BINARY-SOURCE`
(the scope of binary distribution review). Package manifests record
`legal_clearance: false`.

See the [first-release review](release-rights-review.md) for the role of each
unresolved DoomGeneric file and the evidence needed for the Windows ZIP.

## Distribution records

Keep the license texts, attribution, [modification notices](../../MODIFICATIONS.md),
font sources and generators with the corresponding material. The offline source
archive is paired with the Windows build by SHA-256. Assess the applicable source,
runtime and notice obligations for the intended distribution.

See [distribution](../publication.md), [package contents](../packages.md),
[verification](verification.md), and [image notices](../images/LICENSE.md).
