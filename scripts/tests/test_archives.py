# SPDX-License-Identifier: GPL-3.0-only
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import warnings
import zipfile
from contextlib import contextmanager


@contextmanager
def workspace_temporary_directory():
    root = Path('target/package-tests').resolve()
    root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=root) as directory:
        try:
            yield directory
        finally:
            if not Path(directory).resolve().is_relative_to(root):
                raise ValueError('Temporary cleanup escaped test workspace')

spec = importlib.util.spec_from_file_location('verify_release', Path(__file__).parents[1] / 'verify-release.py')
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class ArchiveTests(unittest.TestCase):
    def make_archive(self, directory, *, content=b'content', record_content=b'content', extra=None):
        path = Path(directory) / 'archive.zip'
        manifest = {'kind': 'symbols', 'legal_clearance': True, 'files': [
            {'path': 'file.txt', 'sha256': hashlib.sha256(record_content).hexdigest(), 'bytes': len(record_content)}]}
        with zipfile.ZipFile(path, 'w') as archive:
            archive.writestr('file.txt', content)
            archive.writestr('ARTIFACT-MANIFEST.json', json.dumps(manifest))
            if extra:
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore')
                    archive.writestr(extra, b'bad')
        return path

    def test_valid_archive_is_read_without_extraction(self):
        with workspace_temporary_directory() as directory:
            archive = self.make_archive(directory)
            verifier.verify_archive(archive)
            self.assertEqual([p.name for p in Path(directory).iterdir()], ['archive.zip'])

    def test_changed_payload_fails_hash_check(self):
        with workspace_temporary_directory() as directory:
            with self.assertRaisesRegex(ValueError, 'Hash mismatch'):
                verifier.verify_archive(self.make_archive(directory, content=b'changed'))

    def test_unlisted_extra_file_is_rejected(self):
        with workspace_temporary_directory() as directory:
            with self.assertRaisesRegex(ValueError, 'count differs'):
                verifier.verify_archive(self.make_archive(directory, extra='extra.txt'))

    def test_duplicate_and_case_colliding_names_are_rejected(self):
        for name in ['file.txt', 'FILE.TXT']:
            with self.subTest(name=name), workspace_temporary_directory() as directory:
                with self.assertRaisesRegex(ValueError, 'Duplicate'):
                    verifier.verify_archive(self.make_archive(directory, extra=name))

    def test_unsafe_paths_are_rejected(self):
        for name in ['../escape', '/absolute', 'C:/drive', 'a\\b', 'a/./b', 'trailing.']:
            with self.subTest(name=name):
                with self.assertRaises(ValueError):
                    verifier.safe_name(name)

    def test_traversal_archive_is_rejected_before_manifest(self):
        with workspace_temporary_directory() as directory:
            with self.assertRaisesRegex(ValueError, 'Unsafe'):
                verifier.verify_archive(self.make_archive(directory, extra='../escape'))


if __name__ == '__main__':
    unittest.main()
