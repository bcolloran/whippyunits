#!/usr/bin/env python3
"""Run the same positive integration checks on a selected compiler/backend.

Diagnostic snapshot tests are excluded: independent negative probes verify
rejection meaning without assuming stable/nightly diagnostic text is identical.
This is correctness validation, not a performance measurement.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--toolchain', required=True)
    parser.add_argument('--target', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--native', action='store_true')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RUSTUP_TOOLCHAIN=args.toolchain, CARGO_INCREMENTAL='0',
               CARGO_TARGET_DIR=str(args.target.resolve()))
    native = ['nightly-gca'] if args.native else []
    def features(items):
        return ['--features', ','.join(items + native)] if items or native else []
    cargo = ['cargo', '--offline', '--locked']
    def cmd(verb, *items):
        return [cargo[0], verb, *cargo[1:], '-j2', *items]
    checks = [
        ('core-narrow', cmd('test', '-p', 'whippyunits-core', '--lib', '--no-default-features', *features([]))),
        ('core-wide', cmd('test', '-p', 'whippyunits-core', '--lib', *features([]))),
        ('quantity', cmd('test', '-p', 'whippyunits', '--lib', '--test', 'custom_storage_types',
            '--test', 'exponent_features', '--test', 'phantom_marker_traits', '--test', 'catalog_features',
            '--no-default-features', *features(['std', 'units-dynamics-conversions']), '--',
            '--skip', 'disabled_catalog_items_are_not_exported', '--skip', 'coherent_catalog_omits_noncoherent_declarators')),
        ('generic-example', cmd('run', '-p', 'whippyunits', '--example', 'centripetal_acceleration', *features([]))),
        ('algebra', cmd('test', '-p', 'whippyalgebra', '--lib', '--test', 'uniform', '--test', 'brands',
            '--test', 'generic_block', '--test', 'generic_matrix_uniform', '--test', 'operations',
            '--test', 'canonical', '--no-default-features', *features(['std', 'nalgebra', 'units-dynamics-conversions']))),
        ('no-std', cmd('check', '-p', 'whippyalgebra', '--lib', '--no-default-features',
            *features(['nalgebra', 'units-dynamics-conversions']))),
    ]
    if args.native:
        checks.insert(0, ('native-numerals', cmd('test', '-p', 'whippyunits-num',
            '--features', 'nightly-gca')))
    probe_features = ['whippyunits/nightly-gca', 'whippyalgebra/nightly-gca'] if args.native else []
    checks.append(('runtime-probe', cmd('run', '--manifest-path', 'probe/Cargo.toml',
        *(['--features', ','.join(probe_features)] if probe_features else []))))
    report = {'scope': 'correctness only; no timing claims', 'status': 'running', 'all_passed': False,
              'planned_checks': [label for label, _ in checks], 'repo': str(args.repo.resolve()),
              'toolchain': args.toolchain, 'native': args.native,
              'rustc': subprocess.check_output(['rustc', '-Vv'], env=env, text=True),
              'environment': {key: env[key] for key in ('RUSTUP_TOOLCHAIN', 'CARGO_INCREMENTAL', 'CARGO_TARGET_DIR')},
              'checks': []}
    for label, command in checks:
        log = args.output / (label + '.log')
        with log.open('w') as stream:
            try:
                result = subprocess.run(command, cwd=args.repo, env=env, stdout=stream,
                                        stderr=subprocess.STDOUT, timeout=300)
                status = result.returncode
            except subprocess.TimeoutExpired:
                status = 'timeout'
        row = {'label': label, 'command': command, 'status': status, 'log': str(log.resolve())}
        report['checks'].append(row)
        report['all_passed'] = len(report['checks']) == len(checks) and all(row['status'] == 0 for row in report['checks'])
        if len(report['checks']) == len(checks):
            report['status'] = 'complete'
        (args.output / 'results.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(row), flush=True)
    return 0 if report['all_passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
