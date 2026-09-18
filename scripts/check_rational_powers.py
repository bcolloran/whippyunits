#!/usr/bin/env python3
"""Check const overflow parity against the actual compiled crate and legacy macros.

Suggested CI commands (after corresponding runtime tests):
  python3 scripts/check_rational_powers.py --overflow-checks on
  python3 scripts/check_rational_powers.py --overflow-checks off
  python3 scripts/check_rational_powers.py --release --overflow-checks on
  python3 scripts/check_rational_powers.py --release --overflow-checks off

Cargo's JSON artifact messages choose the exact dependency variants, rather than
selecting potentially stale files from target/. No production code is copied.
"""
import argparse
import ast
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile


def checked_value(base, exponent):
    """Independent mathematical oracle for the public bounded lookup contract."""
    if abs(exponent) > 100:
        return (1, 1)
    value = pow(base, abs(exponent))
    value = ((value + (1 << 127)) % (1 << 128)) - (1 << 127)
    return (1, value) if exponent < 0 else (value, 1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest-path', type=Path,
                        default=Path(__file__).resolve().parents[1] / 'Cargo.toml')
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--overflow-checks', choices=['on', 'off'], required=True)
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()

    env = os.environ.copy()
    if 'CARGO_ENCODED_RUSTFLAGS' in env:
        rustflags = env['CARGO_ENCODED_RUSTFLAGS'].split('\x1f')
        rustflags = [flag for flag in rustflags if flag]
    else:
        rustflags = shlex.split(env.get('RUSTFLAGS', ''))
    rustflags += ['-C', 'overflow-checks=' + args.overflow_checks]
    env.pop('RUSTFLAGS', None)
    env['CARGO_ENCODED_RUSTFLAGS'] = '\x1f'.join(rustflags)
    command = [env.get('CARGO', 'cargo'), 'test', '--manifest-path',
               str(args.manifest_path.resolve()), '--locked', '-p', 'whippyunits',
               '--test', 'rational_power_tables', '--no-run', '--no-default-features',
               '--features', 'std,units-dynamics', '--message-format=json']
    if args.release:
        command.append('--release')
    if args.offline:
        command.append('--offline')
    build = subprocess.run(command, env=env, text=True, capture_output=True)
    if build.returncode:
        print(build.stderr)
        print(build.stdout)
        raise SystemExit(build.returncode)
    library = macro = None
    for line in build.stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get('reason') != 'compiler-artifact':
            continue
        target = message['target']['name']
        for filename in message['filenames']:
            if target == 'whippyunits' and filename.endswith('.rlib'):
                library = Path(filename)
            if target == 'whippyunits_proc_macros' and Path(filename).suffix in ['.so', '.dylib', '.dll']:
                macro = Path(filename)
    if library is None or macro is None:
        raise RuntimeError('Cargo did not report the required library and proc-macro artifacts')

    # Explicitly configure both crates and the consumer, including overrides that
    # intentionally disagree with the usual debug/release overflow defaults.
    rustc = [env.get('RUSTC', 'rustc'), '--edition=2024', '--crate-name', 'rational_const_probe',
             '-C', 'opt-level=' + ('3' if args.release else '0'), *rustflags,
             '--extern', 'whippyunits=' + str(library),
             '--extern', 'whippyunits_proc_macros=' + str(macro)]
    for directory in sorted({library.parent, macro.parent}):
        rustc += ['-L', 'dependency=' + str(directory)]

    fixtures = [
        ('zero', 5, 0),
        ('safe-two-positive', 2, 100), ('safe-two-negative', 2, -100),
        ('safe-three-positive', 3, 80), ('safe-three-negative', 3, -80),
        ('safe-five-positive', 5, 54), ('safe-five-negative', 5, -54),
        ('overflow-three-positive', 3, 81), ('overflow-three-negative', 3, -81),
        ('overflow-five-positive', 5, 55), ('overflow-five-negative', 5, -55),
        ('wrapped-three-far', 3, 100), ('wrapped-five-far', 5, -100),
        ('fallback-positive', 5, 101), ('fallback-negative', 3, -101),
        ('fallback-min', 3, -(1 << 31)), ('fallback-max', 5, (1 << 31)-1),
    ]
    results = []
    with tempfile.TemporaryDirectory(prefix='whippyunits-const-powers-') as temp:
        directory = Path(temp)
        for label, base, exponent in fixtures:
            overflows = abs(exponent) <= 100 and pow(base, abs(exponent)) > (1 << 127)-1
            expect_success = args.overflow_checks == 'off' or not overflows
            # Separate fixtures prove both legacy/current failures instead of
            # accepting a compilation failure caused by only one implementation.
            for variant in ['legacy', 'current']:
                definition = (
                    f'whippyunits_proc_macros::pow_lookup!(power, {base}, 100, rational);'
                    if variant == 'legacy' else
                    f'use whippyunits::scale_conversion::pow{base} as power;'
                )
                fixture = directory / 'probe.rs'
                executable = directory / ('probe.exe' if os.name == 'nt' else 'probe')
                fixture.write_text(definition + '\n'
                    + f'const VALUE: (i128, i128) = power({exponent});\n'
                    + 'fn main() { println!("{VALUE:?}"); }\n')
                compilation = subprocess.run(rustc + [str(fixture), '-o', str(executable)],
                                             env=env, text=True, capture_output=True)
                success = compilation.returncode == 0
                if success != expect_success:
                    raise AssertionError(f'{label}/{variant}: expected success={expect_success}\n'
                                         + compilation.stderr)
                if success:
                    printed = subprocess.check_output([str(executable)], text=True)
                    value = ast.literal_eval(printed.strip())
                    if value != checked_value(base, exponent):
                        raise AssertionError(f'{label}/{variant}: wrong value {value}')
                elif 'E0080' not in compilation.stderr:
                    raise AssertionError(f'{label}/{variant}: failed for a non-const-overflow reason\n'
                                         + compilation.stderr)
                results.append({'case': label, 'variant': variant, 'success': success})
    summary = {'release': args.release, 'overflow_checks': args.overflow_checks,
               'library': str(library), 'macro': str(macro), 'results': results}
    if args.output:
        args.output.write_text(json.dumps(summary, indent=2) + '\n')
    print(f'Passed {len(results)} const cases: release={args.release}, '
          f'overflow-checks={args.overflow_checks}')


if __name__ == '__main__':
    main()
