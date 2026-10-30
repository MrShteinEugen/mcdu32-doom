# Publication and rights review

This is a technical provenance/compliance preparation, not a legal opinion or a
guarantee against claims. No engineering change can ensure that nobody will
assert copyright, trademark, patent, contractual or trade-dress rights. Legal
outcomes depend on jurisdiction, distribution and facts. A disclaimer cannot
replace permission, and open source does not mean that work has no owner.

Current status: **prepared for review; rights items remain unresolved**, for
source publication as well as a combined binary. Read [license scope](../LICENSE-POLICY.md),
[modification notice](../MODIFICATIONS.md), [Russian assessment](licensing/README.ru.md)
and [machine-readable register](licensing/review-register.json). No upstream
clarification has been fabricated or requested on the owner's behalf.

## License decision

New application releases use **GPL-3.0-only**, not a proprietary/MIT claim
over the whole executable. Most engine files are GPL-2.0-or-later; the included
GnuPG SHA-1 implementation is GPL-3.0-or-later. These explicit grants permit
a GPLv3 combination. Upstream's root GPLv2 notice and six files without
individual version grants need interpretation/clarification, as recorded in the
[2026-10-20 audit](legal-review-2026-10-20.md); this is not silently treated as
an affirmative GPLv3 grant for every line. The protocol reference is
GPLv3, so the new host release uses GPLv3 conservatively. The combined release
does not assert an automatic grant under future GPL versions where an upstream
grant has not established that option. Original engine copyrights/licenses
remain intact; the font map remains BSD-3-Clause. Prior
license grants are not retroactively revoked. Contributors must only submit
material they are entitled to license; do not invent author names or remove
upstream notices. The repository owner should identify the actual release owner
and contact when publishing; no owner identity or GitHub URL is fabricated here.

## Original new functionality

The aircraft state machine in `src/mcdu/` stores local user inputs. Its visual
templates now follow FlyByWire's MCDU labels and layout; original FlyByWire fonts
are bundled. This replaces the earlier independent DEMO presentation. Preserve
the FBW/template and Tyler Knox/font attribution in NOTICE.md and
`assets/aircraft-font/NOTICE.md`, the upstream license, and both original SFD/TTF
font sources. The pinned revision is recorded there. Rebuild instructions for
the derivative bitmaps are in `scripts/build-aircraft-font.py` (requires Pillow).
No screenshots, aircraft performance tables, AIRAC data, flight manuals or
simulator flight logic are included. GPL compliance does not establish a grant
of trademark rights or immunity from every interface/trade-dress claim.

## Reproducible source-only archive

Run from PowerShell:

```powershell
.\scripts\validate-docs.ps1
.\scripts\validate-licenses.ps1
.\scripts\prepare-publication.ps1
```

The script creates a fresh directory and ZIP in `dist/`, using a source allowlist.
It inventories the locked Cargo graph for `x86_64-pc-windows-msvc`, rejects unreviewed license declarations,
copies available dependency license/notice texts, records source SHA-256 values,
and scans files for several common token/private-key patterns. It excludes
executables, WADs, firmware, research downloads, logs, saves, local preferences,
environment files and build products. This limited pattern scan is not a full
secret audit: review the actual archive and Git history. A clean ZIP cannot
remove a secret already committed to Git; revoke exposed credentials and use
GitHub's secret scanning/push protection. The ZIP is **source only**, not proof
that a future binary release complies with every applicable license.

The audit recognizes the exact declarations currently in Cargo.lock, preserving
the expressions rather than silently relabeling dependencies. For dual choices,
permissive alternatives such as MIT are available where declared. Any new/null
declaration stops the script for review. Metadata and copied license files still
need human review against actual shipped code, including native bundled code.
`DEPENDENCIES.json` and copied notices cover the selected Windows build target.
`OTHER-TARGET-DECLARATIONS.json` records unused platform packages as metadata
only: their source/texts are not redistributed by this source archive. The checked-in
review covers only `x86_64-pc-windows-msvc`; another `-Target` is rejected until
a corresponding review/inventory is supplied. Do not treat a target flag as an audit.

`validate-licenses.ps1` compares the actual default-feature locked Windows graph
to `docs/licensing/dependencies-windows.json`, verifies cached registry archives
against Cargo.lock, and checks selected features, declared/selected licenses,
root/nested notice hashes, seven original license/credit texts, selected native
files and the complete vendored engine file set. This is not a byte audit of
all expanded dependency source or a separate all-feature/all-target release.
The wrapper's package license and HIDAPI native/file exceptions are distinguished.
Unknown/changed records stop packaging rather than being silently accepted.
The review snapshot is updated only after reviewing the change, not as a way to
make a failing check green. Native notice paths are preserved when copied.

After creating the ZIP, preparation calls `scripts/verify-publication.ps1` to
read the archive without extracting/executing it, verify every manifest hash,
required license/font-source files, dependency inventory and 114 notice hashes,
excluded runtime paths and the unresolved review status. To repeat that inspection:

```powershell
.\scripts\verify-publication.ps1 -Archive .\dist\YOUR-SOURCE-ARCHIVE.zip
```

Replace the filename with the actual generated path. Hashes are integrity checks,
not a digital signature or proof of the licensor's authority.

The current image set has its own attribution/license notice, pinned IWAD hash,
source-frame metadata and PNG SHA-256 values. It uses Freedoom, not commercial
DOOM artwork. `validate-docs.ps1` checks local links, all 16 published images,
and recorded engine/font hashes. It does not decide trademark or license-grant
ambiguities. The archive manifest points to the dated rights review and lists
remaining review items; `legal_clearance` deliberately remains false.
The checker reports unresolved items even when technical checks pass. Original
license text hashes and the policy are explanatory safeguards, not a grant
from another rights holder. See [unsent clarification questions](licensing/upstream-questions.md).

## Before uploading

1. Review NOTICE.md, THIRD_PARTY.md and the generated license inventory. Resolve
   remaining grant/version ambiguities. The engine revision has now been
   recovered by byte comparison; that identifies provenance rather than proving
   all underlying rights. Preserve the per-file audit and font source notices.
2. Identify the release owner/contact and use an independent project name and
   imagery. Do not claim affiliation, certification or exclusive ownership of
   third-party work. Add genuine author attribution via version-control history.
3. Confirm that the ZIP contains no personal flight plans, simulator assets,
   device firmware, commercial WADs or credentials. This script does not publish
   anything to GitHub and does not make old Git history safe automatically.
4. When distributing binaries, supply their exact corresponding source and build
   scripts under the applicable GPL terms, along with every required dependency
   license/copyright notice. Include source for distributed modified/native
   components as required; do not assume a Cargo.lock or a generic download URL
   alone satisfies all source obligations. Preserve Freedoom notices if bundling
   its data. This source-only preparation deliberately does not bundle game data.

## Primary references reviewed

- [GNU GPLv3](https://www.gnu.org/licenses/gpl-3.0.html), especially sections 4–6.
- [GNU license FAQ](https://www.gnu.org/licenses/gpl-faq.html).
- [MCDU menu functionality](https://docs.flybywiresim.com/pilots-corner/a32nx/a32nx-briefing/mcdu/mcdu-menu/),
  [INIT](https://docs.flybywiresim.com/pilots-corner/a32nx/a32nx-briefing/mcdu/init/),
  [PROG](https://docs.flybywiresim.com/pilots-corner/a32nx/a32nx-briefing/mcdu/prog/),
  [PERF](https://docs.flybywiresim.com/pilots-corner/a32nx/a32nx-briefing/mcdu/perf/).
- [GitHub secret scanning](https://docs.github.com/en/code-security/concepts/secret-security/secret-scanning).
