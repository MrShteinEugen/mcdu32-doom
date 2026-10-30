# License scope and distribution policy

The original application contributions are offered under **GNU GPL version 3
only (SPDX: GPL-3.0-only)**, to the extent their contributors have rights to grant.
The unmodified license is [LICENSE](LICENSE). This policy explains scope; it
does **not** amend GPL, impose additional restrictions, transfer anyone else's
copyright, or warrant that all third-party rights have been cleared.

## What the root license covers

New Rust host code, C host bridge, new scripts, examples, tests and original
documentation/presentation contributions use GPL-3.0-only. Existing individual
notices and third-party grants take precedence for their respective material.
Applying GPLv3 to a permitted combination does not erase the original grants.
In particular, recipients retain the original GPL-2.0-or-later and
GPL-3.0-or-later choices for upstream portions so licensed. License documents
themselves retain their own copying permissions.

| Material | Governing record |
| --- | --- |
| DOOM/Chocolate Doom engine | Individual notices in `vendor/doomgeneric`, upstream GPLv2 text, [file audit](docs/engine-provenance.json). Most files grant GPL-2.0-or-later; `sha1.c` grants GPL-3.0-or-later. Six files have unresolved project-level version interpretation. |
| Host changes to engine files | [Dated modification record](MODIFICATIONS.md); underlying notices preserved. Host additions are GPL-3.0-only; that grant cannot enlarge upstream permissions. |
| Captured font transaction map | [Andrew Whewell's complete BSD-3-Clause notice](assets/font-map-LICENSE.txt). |
| Aircraft fonts and derived bitmap | [Original sources and upstream GPLv3 notice](assets/aircraft-font/NOTICE.md). No font exception is assumed. |
| Aircraft page adaptations | FlyByWire GPLv3 reference, credited in [THIRD_PARTY.md](THIRD_PARTY.md); new host contributions GPL-3.0-only. |
| Images | [Per-image scope and notices](docs/images/LICENSE.md), including underlying Freedoom BSD-3-Clause artwork. |
| Freedoom data | [BSD notice](assets/freedoom-LICENSE.txt) and [credits](assets/freedoom-CREDITS.txt). IWAD is downloaded separately, not included in source releases. |
| Rust/native dependencies | [Locked Windows inventory](docs/licensing/dependencies-windows.json). Their own grants and notices apply; Cargo metadata alone is not a file-level license conclusion. |

The host's `Cargo.toml` license field describes the original application
license. It is not a claim that all repository bytes have one license. No MIT,
public-domain, proprietary or exclusive-ownership grant is made over the engine,
third-party fonts, game data or other people's contributions.

## Current publication review status

**Prepared for review; legal clearance has not been established.**
[Review register](docs/licensing/review-register.json) records unresolved matters,
evidence and resolution criteria. Posting source to GitHub is distribution too;
withholding binaries does not cure a missing source redistribution permission.
An automated check can verify notices and known bytes, not invent a grant.
Do not describe a successful check, ZIP manifest, GitHub license badge or this
document as legal approval. See the [practical Russian assessment](docs/licensing/README.ru.md).

The six unresolved engine files are not silently marked GPL-2.0-only or
GPL-2.0-or-later. A root GPLv2 text does not by itself settle every question of
scope/version. If relevant material is GPL-2.0-only, combining it with GPLv3-only
material is incompatible absent another permission. Obtain and retain evidence
or a qualified assessment before representing the combined release as cleared.
The [unsent questions](docs/licensing/upstream-questions.md) identify the issue.

## Source and binary distribution

For source, retain the applicable license texts, copyright, warranty notices,
attribution and modification notices. Keep font sources/generators and image
notices with their derivatives. The allowlisted archive retains these materials;
it does not publish them automatically. Review the exact files and any existing
Git history, including deleted files, before pushing.

For a binary release, assess the exact build, target, features, linked native
code and distribution method. GPLv3 sections 1 and 6 define Corresponding Source
and the permitted methods of providing it. A practical GitHub download arrangement
is equivalent access beside each binary to its complete corresponding source,
notices and necessary build scripts under the applicable terms. Retain immutable
release identifiers. The current source-only ZIP includes application/engine/font
source and dependency notices, **not all dependency source**; it must not be
advertised as a verified complete Corresponding Source bundle for binaries.
Determine dependency and toolchain/runtime obligations for the actual build.
System Library exclusions are fact-specific, not a blanket exclusion for C code.

If distributing a covered User Product in circumstances specified by GPLv3,
assess Installation Information as well. This host does not ship panel firmware
or locked-down hardware. That fact alone is not an assessment of every future
product or installation arrangement. Preserve the appropriate legal notices in
interactive interfaces where required by GPLv3 section 5(d); audit an actual
binary release separately rather than treating this source preparation as that audit.

## Rights outside the code license

No endorsement or trademark license is asserted for DOOM, Airbus/A320, FlyByWire,
Honeywell or WINCTRL/WINWING. Names describe the software, compatibility
or references. GPLv3 section 7 permits preserving specified attribution and
withholding trademark permissions; this policy adds no trademark permission or
new restriction to GPL-covered code. Existing trademark rights remain separate.

Commercial DOOM WAD files, logos/art, aircraft databases, simulator executables,
device firmware, manuals and third-party photographs are not authorized by the
root GPL. Do not add them on the assumption that the engine license covers them.
Screenshots currently use verified Freedoom data. A user's right to play their
own WAD is not automatically a right to redistribute it or its artwork.

Interface appearance, game appearance, contractual restrictions, patents,
jurisdiction-specific law and contributors' authority require separate assessment.
GPLv3 contains limited patent provisions; these are not universal third-party
patent clearance. Do not add a noncommercial-only, no-modification, no-lawsuit
or no-redistribution clause: these would conflict with freedoms granted by GPL.
Its warranty/liability provisions do not prevent claims or override mandatory law.

## Contributions

Contributors keep their rights and must have authority to grant the applicable
license. No copyright assignment is required or implied. Original new
contributions use GPL-3.0-only; third-party adaptations retain their underlying
grants and provenance. See [CONTRIBUTING.md](CONTRIBUTING.md). Do not invent an
author identity, claim an employer's rights without authority, remove notices,
or claim exclusive copyright over third-party or automatically generated material.

## Authoritative references

- [GPLv3](https://www.gnu.org/licenses/gpl-3.0.html), sections 1, 4–7, 10–12 and 15–17.
- [GPLv2](https://www.gnu.org/licenses/old-licenses/gpl-2.0.html), especially section 9.
- [GNU compatibility FAQ](https://www.gnu.org/licenses/gpl-faq.html#AllCompatibility).
- [GNU font FAQ](https://www.gnu.org/licenses/gpl-faq.html#FontException).
- [GitHub repository licensing](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository).

This policy and translations are explanatory. The applicable original license
texts and actual grants control; no translation or audit summary replaces them.
