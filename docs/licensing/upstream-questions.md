# Unsent permission-clarification questions

These are drafts for the release owner or counsel. They have **not** been sent.
No author/maintainer is assumed to own every contribution merely because they
maintain a repository. Obtain confirmation from the relevant rights holders or
an assessment supported by the contribution history. Preserve actual responses,
their scope and any conditions; do not put private correspondence in public Git
without checking permission and personal information.

## DoomGeneric scope/version

Subject: License scope clarification for selected DoomGeneric files

We are preparing a Rust MCDU host using a selected DoomGeneric source snapshot,
revision `e257a09527062bfb986b6e5e0780839b6cbd5b71`. Most selected files explicitly
grant GPL-2.0-or-later, and `sha1.c` grants GPL-3.0-or-later. Our host and aircraft
font contributions are distributed under GPLv3 terms. We retain all notices.

The following unchanged files have no individual version/grant notice:
`doomgeneric/config.h`, `doomgeneric/doom.h`, `doomgeneric/doomgeneric.c`,
`doomgeneric/doomgeneric.h`, `doomgeneric/dummy.c`, `doomgeneric/mus2mid.h`.
The repository root contains the GPLv2 text. Could you clarify:

1. Which license/version grant applies to these particular files at this
   revision, including whether GPLv2-or-later or GPLv3 use is authorized?
2. Who owns the relevant portions, including the Florian-attributed files and
   inherited/generated headers? What history or upstream grant supports this?
3. Can the applicable clarification or permission be recorded publicly, with
   its exact scope and the authorized rights holders identified?

We are not asking you to license someone else's work without authority. If
multiple holders are involved, identifying the source/history would help.
We will not change existing notices or interpret a generic acknowledgment as a
new permission. We can supply the per-file hashes and exact downstream diff.

## Questions for the release owner's IP review

- Does the project-level DoomGeneric evidence establish GPLv3-compatible
  redistribution of the selected source and combination? If not, what written
  permission is required? Assess source publication as well as binary linking.
- Does the actual aircraft UI/branding or Blocks presentation raise separate
  copyright/trademark/trade-dress issues in the intended jurisdictions?
- Do the owner's hardware/software agreements impose relevant limits on the
  protocol implementation or distribution? No firmware/vendor program is shipped.
- Are there employment/contractor rights or other contributors to account for?
  What attribution/contact can legitimately be public? Assess AI-assisted work
  under applicable law without assuming a blanket copyright or originality rule.
- If releasing binaries, are the proposed Corresponding Source, notices,
  interactive legal notices and any Installation Information sufficient for
  that exact distribution arrangement?
