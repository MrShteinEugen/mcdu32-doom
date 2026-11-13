# Contributing

The supported development target is Windows with the MSVC Rust toolchain and
Visual Studio C++ Build Tools. The DOOM engine is built from the checked-in C
sources; no external engine checkout is required.

```powershell
.\scripts\setup-freedoom.ps1
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
.\scripts\validate-docs.ps1
.\scripts\validate-licenses.ps1
cargo build --release --locked
```

Run the game-data tests explicitly after installing Freedoom:

```powershell
cargo test --release --locked --test cli --test engine_smoke --test engine_ui --test emulator_smoke --test service --test mcdu_service -- --ignored --test-threads=1
python -B -m unittest discover -s scripts/tests -v
.\scripts\check-powershell.ps1
```

These targets use synthetic input and do not access USB. Test outputs are stored
under `target/` and `output/`. A missing IWAD fails an explicitly selected fixture
test. See [CI](docs/ci.md) and [package verification](docs/packages.md).

Keep hardware changes separate from visual or gameplay changes. Font packing,
glyph addresses, grid commands and packet timing have physical consequences.
`tests/render_contract.rs` protects the approved font transactions and fixed-frame
cell output for all three display profiles and both rendering styles. Update its
fingerprints only for an intentional behavior change, with a reviewed comparison.

`tests/engine_ui.rs` drives actual DOOM through synthetic MCDU input reports. Set
`MCDU_UI_CAPTURE_DIR` to an output directory to save its gameplay, menu and message
images. A host-side test cannot prove firmware acceptance; record physical
observations separately when changing the protocol.

New host releases use GPL-3.0-only. Most engine files retain GPL-2.0-or-later;
the GnuPG SHA-1 file is GPL-3.0-or-later. Preserve upstream project notices and
the [recorded grant/version ambiguities](docs/licensing/review-register.json). Preserve third-party
notices and document modifications in [THIRD_PARTY.md](THIRD_PARTY.md). Game WADs,
build products, logs, local research checkouts and savegames are excluded from Git.

Regenerate publication images with `scripts/capture-readme.ps1`. Keep the image
license, source metadata and hashes together. Do not replace Freedoom images with
commercial game assets without reviewing the different rights and updating attribution.

Read [license scope](LICENSE-POLICY.md) and keep the [modification record](MODIFICATIONS.md)
current for adaptations of third-party material. Every original new contribution
is submitted under GPL-3.0-only to the extent the contributor has authority;
adaptations preserve underlying licenses and provenance. No copyright assignment
or grant over someone else's work is implied. Identify copied/adapted/generated
material and its exact source and permissions. AI assistance does not establish
uniqueness or licensing authority; review its output as you would other code.

For contributions, use a real, authorized Git author identity and
`git commit --signoff` to certify the [Developer Certificate of Origin 1.1](https://developercertificate.org/).
The sign-off certifies origin/authority; it does not transfer copyright, supply
an absent upstream grant or guarantee non-infringement. Do not forge past
contributors' sign-offs or identities. Contributions without this attestation
need origin/authority review before acceptance.

`scripts/validate-licenses.ps1` compares the checked-in review to the actual locked
Windows graph, nested notice hashes, selected native files and engine provenance.
Review dependency or license changes before updating the inventory. See the
[license policy](LICENSE-POLICY.md) for the scope of the review.
