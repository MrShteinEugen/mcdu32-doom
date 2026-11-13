# License scope and distribution

Original application contributions are licensed under GNU GPL version 3 only
(SPDX: GPL-3.0-only), to the extent their contributors have authority to grant
those rights. [LICENSE](LICENSE) contains the original license text.
Third-party files retain their individual notices and grants.

## Components

| Material | Governing record |
| --- | --- |
| Rust host, C host bridge, scripts, tests and original documentation | GPL-3.0-only |
| DOOM/Chocolate Doom engine | Individual notices, upstream GPLv2 text and [file audit](docs/engine-provenance.json) |
| Host adaptations of engine files | [Modification notice](MODIFICATIONS.md), with underlying grants preserved |
| Captured transaction map | [Andrew Whewell's BSD-3-Clause notice](assets/font-map-LICENSE.txt) |
| Aircraft fonts and derived bitmap | [Original font sources and GPLv3 notice](assets/aircraft-font/NOTICE.md); no font exception assumed |
| Aircraft page adaptations | FlyByWire reference and attribution in [THIRD_PARTY](THIRD_PARTY.md) |
| Screenshots | [Image-specific notices](docs/images/LICENSE.md), including Freedoom artwork |
| Freedoom data | [BSD notice](assets/freedoom-LICENSE.txt) and [credits](assets/freedoom-CREDITS.txt); IWAD downloaded separately |
| Rust/native dependencies | [Windows inventory](docs/licensing/dependencies-windows.json) and [native exceptions](docs/licensing/native-components.json) |

Most engine files grant GPL-2.0-or-later; the GnuPG `sha1.c` grants
GPL-3.0-or-later. Their original options remain available for those portions.
The host's package license does not relabel every file in the repository.
License documents retain their own copying permissions.

## Review status

The [review register](docs/licensing/review-register.json) records unresolved
engine grant/version interpretation, presentation and owner/contract matters,
plus the scope of binary-source review. Legal clearance has not been established.
Technical checks verify known bytes and notices, not rights or authority.

Six engine files have no individual version grant: `config.h`, `doom.h`,
`doomgeneric.c`, `doomgeneric.h`, `dummy.c` and `mus2mid.h`. The upstream root
GPLv2 text alone does not resolve their applicable scope/version. GPL-2.0-only
material cannot be combined with GPLv3-only material without another permission;
resolution requires the evidence identified in the register.

## Distribution

Retain applicable license texts, copyright, warranty and attribution notices,
and notices of changes. Include original font sources and bitmap generators
with their derivatives. Review the exact files and Git history being distributed.

For binaries, assess the actual build, target, features, linked native code and
method of providing Corresponding Source under the applicable terms. Supply the
matching source, notices and necessary build scripts alongside the binaries.
The source-only archive contains dependency notices; source-offline additionally
contains locked dependency sources. Rust, MSVC, Windows SDK and system/runtime
components are external prerequisites whose applicable obligations require review.

GPLv3 sections 1 and 6 define Corresponding Source and distribution methods.
Assess System Library exclusions for the actual components rather than treating
all native code as exempt. Installation Information is also required where the
User Product provisions apply. Preserve appropriate interactive legal notices
where required by section 5(d). See [distribution records](docs/publication.md)
and [package contents](docs/packages.md).

## Other rights

Product names identify compatibility and references. No trademark license,
endorsement or certification is asserted for DOOM, Airbus/A320, FlyByWire,
Honeywell or WINCTRL/WINWING. Code licensing does not authorize commercial WADs,
logos, aircraft databases, simulator files, firmware or third-party photographs.
Screenshots use Freedoom with the accompanying notices.

Contributor authority, employment/contract rights, interface presentation,
patents and applicable law require their own assessment. GPL warranty and
liability clauses do not override mandatory law. This policy adds no restrictions
to the freedoms granted by the original licenses and does not amend their terms.

## Contributions and references

Contributors retain their rights and must have authority to grant the applicable
license. No copyright assignment is required. Adaptations preserve the underlying
licenses and provenance; [CONTRIBUTING](CONTRIBUTING.md) describes submission rules.

- [GPLv3](https://www.gnu.org/licenses/gpl-3.0.html), sections 1, 4–7, 10–12 and 15–17.
- [GPLv2](https://www.gnu.org/licenses/old-licenses/gpl-2.0.html), including section 9.
- [GNU compatibility FAQ](https://www.gnu.org/licenses/gpl-faq.html#AllCompatibility).
- [GNU font FAQ](https://www.gnu.org/licenses/gpl-faq.html#FontException).

This policy explains project records; original license texts and actual grants control.
[Component and distribution overview](docs/licensing/README.md).
