# SPDX-License-Identifier: GPL-3.0-only
"""Read-only verification of release archives. Never extracts or executes code."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import struct
import zipfile


def digest(stream):
    return hashlib.file_digest(stream, "sha256").hexdigest()


def safe_name(name):
    path = PurePosixPath(name)
    if (not name or "\\" in name or ":" in name or path.is_absolute()
            or any(p in {".", ".."} or p.endswith((".", " ")) for p in name.split("/"))):
        raise ValueError(f"Unsafe archive path: {name}")
    return name


def verify_archive(path):
    with zipfile.ZipFile(path) as archive:
        entries = {}
        for entry in archive.infolist():
            name = safe_name(entry.filename)
            if name.split('/')[0] in {'.git', 'data', 'output', 'research', 'target', 'dist'}:
                raise ValueError(f'Runtime directory in archive root: {name}')
            if name.casefold() in entries or entry.is_dir() or entry.file_size > 256 * 1024**2:
                raise ValueError(f"Duplicate, directory or oversized entry: {name}")
            if (entry.external_attr >> 16) & 0o170000 == 0o120000:
                raise ValueError("Symbolic links are not permitted")
            entries[name.casefold()] = entry
        if sum(e.file_size for e in entries.values()) > 1024**3:
            raise ValueError("Uncompressed archive exceeds 1 GiB")
        manifest_name = "PUBLICATION-MANIFEST.json" if "publication-manifest.json" in entries else "ARTIFACT-MANIFEST.json"
        manifest = json.loads(archive.read(manifest_name))
        if type(manifest["legal_clearance"]) is not bool:
            raise ValueError("Expected a Boolean legal-clearance status")
        if len(entries) != len(manifest["files"]) + 1:
            raise ValueError("Manifest file count differs")
        seen = set()
        for record in manifest["files"]:
            name = safe_name(record["path"])
            key = name.casefold()
            if key in seen or name == manifest_name or key not in entries:
                raise ValueError("Duplicate or missing manifest entry")
            seen.add(key)
            with archive.open(entries[key]) as stream:
                if digest(stream) != record["sha256"]:
                    raise ValueError(f"Hash mismatch: {name}")
            if "bytes" in record and record["bytes"] != entries[key].file_size:
                raise ValueError("Size mismatch")
        if manifest["kind"] not in {"portable", "symbols", "source-only", "source-with-dependencies"}:
            raise ValueError("Unsupported archive kind")
        if manifest["kind"] == "portable":
            for binary, subsystem in [("mcdu-doom.exe", 3), ("mcdu-doom-tray.exe", 2)]:
                data = archive.read(binary)
                offset = struct.unpack_from("<I", data, 0x3C)[0]
                if data[:2] != b"MZ" or data[offset:offset+4] != b"PE\0\0":
                    raise ValueError("Not a PE executable")
                if struct.unpack_from("<H", data, offset+4)[0] != 0x8664 or struct.unpack_from("<H", data, offset+24+68)[0] != subsystem:
                    raise ValueError("Wrong executable architecture/subsystem")
            for required in ["BUILD-INFO.json", "SBOM.cdx.json", "DEPENDENCIES.json", "LICENSE", "START-HERE.md", "scripts/setup-freedoom.ps1"]:
                if required.casefold() not in entries:
                    raise ValueError(f"Missing portable file: {required}")
        return manifest


def verify_release(directory):
    release = json.loads((directory / "RELEASE-MANIFEST.json").read_text(encoding="utf-8-sig"))
    if type(release["legal_clearance"]) is not bool or len(release["artifacts"]) != 4:
        raise ValueError("Unexpected release manifest")
    expected = set()
    source_hash = next(a["sha256"] for a in release["artifacts"] if a["name"].endswith("source-offline.zip"))
    for artifact in release["artifacts"]:
        name = safe_name(artifact["name"])
        if "/" in name or name in expected:
            raise ValueError("Invalid artifact name")
        expected.add(name)
        path = directory / name
        with path.open("rb") as stream:
            if digest(stream) != artifact["sha256"] or path.stat().st_size != artifact["bytes"]:
                raise ValueError(f"Artifact hash/size mismatch: {name}")
        manifest = verify_archive(path)
        if manifest["legal_clearance"] != release["legal_clearance"]:
            raise ValueError("Archive and release clearance statuses differ")
        if manifest["kind"] in {"portable", "symbols"} and manifest["source_archive_sha256"] != source_hash:
            raise ValueError("Binary/source link differs")
    if {p.name for p in directory.glob("*.zip")} != expected:
        raise ValueError("Unexpected or missing ZIP artifacts")
    checksum_names = set()
    for line in (directory / "SHA256SUMS.txt").read_text(encoding="utf-8-sig").splitlines():
        checksum, name = line.split("  ", 1)
        safe_name(name)
        if name in checksum_names:
            raise ValueError("Duplicate checksum entry")
        checksum_names.add(name)
        if "/" in name:
            raise ValueError("Checksum path must be a filename")
        with (directory / name).open("rb") as stream:
            if digest(stream) != checksum:
                raise ValueError(f"SHA256SUMS mismatch: {name}")
    if checksum_names != expected | {"RELEASE-MANIFEST.json", "BUILD-INFO.json"}:
        raise ValueError("Incomplete or unexpected checksum inventory")
    print(f"Verified {len(expected)} archives, every file hash, PE subsystems and source links.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    verify_release(parser.parse_args().directory.resolve())
