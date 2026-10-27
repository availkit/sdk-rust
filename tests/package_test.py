#!/usr/bin/env python3
"""Use actual cargo package --list to prove private files remain outside .crate."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile

source = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('package_preflight', source/'build.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

with tempfile.TemporaryDirectory(prefix='availkit-rust-package-') as directory:
    root = Path(directory)/'source'
    for name in module.PACKAGE_FILES:
        path = root/name
        path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source/name, path)
    injected = ('.local/config.json', 'src/private-config.json',
                'examples/private-intent.json', 'tests/credential.json')
    for name in injected:
        path = root/name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('INJECTED_FIXTURE_SECRET')
    module.check(root)
    result = subprocess.run(['cargo', 'package', '--list', '--locked', '--manifest-path',
                             str(root/'Cargo.toml')], check=True, capture_output=True, text=True)
    names = set(result.stdout.splitlines())
    assert not names.intersection(injected), names.intersection(injected)
    assert names == set(module.PACKAGE_FILES)|{'Cargo.toml.orig'}, names
    selected = root/'src/models.rs'
    selected.unlink()
    selected.symlink_to(source/'src/models.rs')
    try:
        module.check(root)
        raise AssertionError('selected symlink admitted')
    except ValueError as error:
        assert str(error) == 'symlink_package_input'
    selected.unlink()
    shutil.copy2(source/'src/models.rs', selected)
    shutil.move(root/'examples', root/'real-examples')
    (root/'examples').symlink_to(root/'real-examples', target_is_directory=True)
    try:
        module.check(root)
        raise AssertionError('selected parent symlink admitted')
    except ValueError as error:
        assert str(error) == 'symlink_package_input'
print('PASS actual cargo package list, private injection exclusion and selected/parent symlink rejection')
