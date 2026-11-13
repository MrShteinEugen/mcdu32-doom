# License verification

`scripts/validate-licenses.ps1` compares the Windows dependency graph with the
checked-in review. `scripts/verify-publication.ps1` inspects a prepared source
archive without extracting or executing it.

## Checked records

| Record | Validation |
| --- | --- |
| [Windows dependencies](dependencies-windows.json) | 53 packages, selected features, declared/selected licenses and Cargo.lock hash |
| Registry archives | SHA-256 against Cargo.lock |
| Dependency notices | 114 original root/nested notice files |
| [Original license texts](license-texts.json) | Seven license and copyright/credit files |
| [Native components](native-components.json) | Selected HIDAPI native/build files and license exceptions |
| [Engine provenance](../engine-provenance.json) | Complete 179-file inventory and hashes |
| [Asset provenance](../asset-provenance.json) | Four original aircraft font files |

The Windows inventory describes the default-feature build. Other targets and
feature selections require their own review. Package declarations and selected
native-file checks are not a file-by-file audit of every dependency source.

## Archive checks

Source manifests enumerate all included files. Verification checks their hashes,
required notices, dependency inventory, font sources and review-register status.
Source-offline archives also retain Cargo vendor checksums. Portable and symbol
archives identify the matching source-offline ZIP by SHA-256.

The archive test suite rejects changed payloads, unlisted files, duplicate names,
case collisions and parent-directory traversal. Run it with:

```powershell
python -B -m unittest discover -s scripts/tests -v
```

[Package verification](../packages.md) documents archive commands and offline
rebuilds. Hash verification establishes file integrity; it is not a signature
or a grant of rights. The [review register](review-register.json) remains the
source of open items and resolution criteria.
