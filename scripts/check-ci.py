# SPDX-License-Identifier: GPL-3.0-only
"""Repository-specific CI invariants; actionlint checks workflow semantics."""
from pathlib import Path
import re
import yaml


def main():
    files = sorted(Path('.github/workflows').glob('*.yml')) + [Path('.github/actions/setup/action.yml')]
    for path in files:
        text = path.read_text(encoding='utf-8')
        data = yaml.safe_load(text)
        assert isinstance(data, dict), path
        assert 'pull_request_target' not in text and 'secrets: inherit' not in text, path
        for action in re.findall(r'uses:\s*([^\s]+)', text):
            assert action.startswith('./') or re.fullmatch(r'[\w.-]+/[\w./-]+@[0-9a-f]{40}', action), (path, action)
        for name, job in data.get('jobs', {}).items():
            if 'uses' not in job:
                assert job.get('timeout-minutes', 0) > 0, (path, name)
            assert job.get('permissions', {}).get('contents', 'read') == 'read', (path, name)
        assert not re.search(r'\$\{\{\s*github\.(event\.(pull_request|issue)|head_ref)', text), path
    print(f'Validated {len(files)} workflow/action policies and immutable action pins.')


if __name__ == '__main__':
    main()
