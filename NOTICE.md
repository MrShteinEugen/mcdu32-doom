# Notices and attribution

Original application contributions are licensed under GPL-3.0-only. See LICENSE
and [license scope](LICENSE-POLICY.md). Third-party portions retain their own grants;
unresolved scope/version matters are recorded, not overridden by this notice.
There is no warranty, including merchantability or fitness for a particular
purpose. Original contributions are attributed to their contributors; this
notice does not claim exclusive ownership of third-party work or remove any
existing rights. Existing GPL-2.0-or-later grants remain valid for the corresponding code.

DOOM/Chocolate Doom files in `vendor/doomgeneric` retain GPL-2.0-or-later notices,
including id Software and Simon Howard copyrights. `sha1.c` retains its GnuPG
GPL-3.0-or-later notice, Copyright (C) 1998–2001 Free Software Foundation, Inc.
Upstream's GPLv2 root license and files without individual version grants are
preserved; their scope is recorded in the dated audit, not silently relabeled.
The identified GPLv2-or-later/GPLv3-or-later portions can be combined under GPLv3.
WAD files are not included. Freedoom screenshot notices and credits are retained
in assets and docs/images/LICENSE.md.

The mcdu-dotnet font transaction map is copyright Andrew Whewell, 2025 onwards,
BSD-3-Clause. Its full notice is `assets/font-map-LICENSE.txt`. The WINCTRL X-Plane
plugin is a GPLv3 protocol-research reference; its code is not linked into this
application. The general application license is conservatively GPLv3, including
the host's protocol serializer. See THIRD_PARTY.md for sources and modifications.

Aircraft page labels and placement are adapted from FlyByWire MCDU templates;
the HoneywellMCDU and HoneywellMCDUSmall fonts are copyright (c) 2021 Tyler Knox.
The original TTF files, FontForge SFD sources and GPLv3 license are included in
`assets/aircraft-font/`, with their pinned upstream revision and transformation
instructions. Template portions: copyright FlyByWire Simulations, 2021-2023,
2025. No logos, screenshots, navigation databases or simulator flight logic are
included. Pages use local state and are not operational flight-management software.

DOOM, Airbus, A320, FlyByWire, WINCTRL/WINWING names are used only as
descriptive references to their respective products or organizations. This
independent project does not claim their endorsement, certification or ownership
of their marks. No proprietary simulator files or device firmware are included.

Rust dependencies retain their own licenses. The publication preparation script
creates a versioned inventory and copies available license/notice texts into the
source archive. See the [distribution guide](docs/publication.md) for release requirements.

The Windows build statically compiles native HIDAPI carried by hidapi-rs 2.6.7.
Native HIDAPI offers three alternatives; the BSD-3-Clause alternative is selected.
Copyright Alan Ott / Signal 11 Software and libusb/hidapi contributors is retained
in its original notices, copied into THIRD_PARTY_LICENSES/hidapi-2.6.7/etc/hidapi/
by the source publication script. The Rust wrapper's package grant is MIT; its
build.rs also retains an individual GPL-3.0-or-later header. Neither is silently
replaced by the other's declaration. See docs/licensing/native-components.json.

The consolidated [modification notice](MODIFICATIONS.md) is dated 2026-10-20.
The [license policy](LICENSE-POLICY.md) links to the open items in the rights review.
