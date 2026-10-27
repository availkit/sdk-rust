#!/usr/bin/env python3
"""Reject changed package allowlists and selected symlinks before cargo package."""
from pathlib import Path
import stat
import tomllib

PACKAGE_FILES = (
    'Cargo.toml', 'Cargo.lock', 'LICENSE', 'README.md', 'VERSION', 'coverage.json',
    'runtime-policy.json', 'generate.py', 'build.py',
    'src/lib.rs', 'src/models.rs', 'src/methods.rs', 'src/validation.rs',
    'src/relations.rs', 'src/private_files.rs', 'src/schemas.json', 'src/operations.json',
    'examples/first_booking.rs', 'examples/public_booking.rs',
    'tests/transport.rs', 'tests/fixtures.json', 'tests/package_test.py',
    'CHANGELOG.md',
    'SECURITY.md',
    'SUPPORT.md',
    'DEVELOPMENT.md',
    'contract-projection.json',
    'contracts/api.openapi.json',
    'contracts/check_projection.py',
)

def check(root):
    include = tomllib.loads((root/'Cargo.toml').read_text())['package']['include']
    if tuple(include) != PACKAGE_FILES:
        raise ValueError('package_allowlist_mismatch')
    for name in PACKAGE_FILES:
        current = root
        for part in Path(name).parts:
            current /= part
            info = current.lstat()
            if stat.S_ISLNK(info.st_mode):
                raise ValueError('symlink_package_input')
        if not stat.S_ISREG(info.st_mode):
            raise ValueError('nonregular_package_input')

if __name__ == '__main__':
    check(Path(__file__).resolve().parent)
    print('PASS exact Rust package source allowlist and no selected symlink inputs')
