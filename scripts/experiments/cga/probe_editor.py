#!/usr/bin/env python3
"""Check inferred native-numeral hovers in a minimal two-crate workspace.

This is an editor correctness probe, not a latency benchmark. Exit 0 means all
four hovers resolve N<7>; exit 1 records a completed observation with invalid
inference; exit 2 reports a preflight, compiler, or LSP error instead.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import tomllib


REPO = Path(__file__).resolve().parents[3]
SOURCE = '''use whippyunits_num::N;

fn main() {
    let sum = N::<3> + N::<4>; // hover-sum
    let annotated: N<7> = N::<3> + N::<4>; // hover-annotated
    let control = N::<7>; // hover-control
    let difference = N::<9> - N::<2>; // hover-difference
    std::hint::black_box((sum, annotated, control, difference));
}
'''


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--server', type=Path, required=True, help='rust-analyzer executable')
    parser.add_argument('--output', type=Path, required=True, help='Fresh or empty artifact directory')
    parser.add_argument('--toolchain', default='nightly-2026-09-13')
    args = parser.parse_args()
    output = args.output.resolve()
    server = args.server.resolve()
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        parser.error('--output must be absent or empty')
    if not server.is_file():
        parser.error('--server must name an existing executable')
    output.mkdir(parents=True, exist_ok=True)
    project = output / 'project'
    environment = dict(os.environ, RUSTUP_TOOLCHAIN=args.toolchain, CARGO_INCREMENTAL='0',
                       CARGO_TARGET_DIR=str(output / 'target'))
    report = {
        'scope': 'editor correctness only; no latency or memory claims',
        'status': 'preparing', 'valid_inference': None,
        'source_origin': str(REPO / 'whippyunits-num'), 'project': str(project),
        'script_sha256': digest(Path(__file__)),
        'toolchain': args.toolchain, 'server': str(server), 'commands': [], 'hover': {},
        'environment': {key: environment[key] for key in
                        ('RUSTUP_TOOLCHAIN', 'CARGO_INCREMENTAL', 'CARGO_TARGET_DIR')},
    }

    def save(name, data):
        path = output / name
        temporary = path.with_name(path.name + '.tmp')
        temporary.write_text(json.dumps(data, indent=2) + '\n')
        temporary.replace(path)

    client = None
    log = None
    save('results.json', report)
    try:
        (project / 'src').mkdir(parents=True)
        shutil.copytree(REPO / 'whippyunits-num', project / 'whippyunits-num',
                        ignore=shutil.ignore_patterns('target', '.git', '__pycache__'))
        metadata = tomllib.loads((REPO / 'Cargo.toml').read_text())['workspace']['package']
        shared_metadata = '\n'.join(f'{key} = {json.dumps(value)}' for key, value in metadata.items())
        (project / 'Cargo.toml').write_text('''[workspace]
members = ["whippyunits-num"]
[workspace.package]
''' + shared_metadata + '''
[package]
name = "minimal_native_hover"
version = "0.0.0"
edition = "2024"
[dependencies]
whippyunits-num = { path = "whippyunits-num", features = ["nightly-gca"] }
''')
        (project / 'src/main.rs').write_text(SOURCE)
        report['source_hashes'] = {str(path.relative_to(project)): digest(path)
                                   for path in sorted(project.rglob('*')) if path.is_file()}
        report['status'] = 'compiler_preflight'
        for name, command in [
            ('rustc-version', ['rustup', 'run', args.toolchain, 'rustc', '-Vv']),
            ('lockfile', ['cargo', 'generate-lockfile', '--offline']),
            ('cargo-check', ['cargo', 'check', '--offline', '--locked', '-j1']),
        ]:
            row = {'name': name, 'command': command, 'cwd': str(project), 'status': 'running',
                   'log': str(output / (name + '.log'))}
            report['commands'].append(row)
            save('results.json', report)
            result = subprocess.run(command, cwd=project, env=environment, text=True,
                                    capture_output=True, timeout=90)
            Path(row['log']).write_text(result.stdout + result.stderr)
            row.update(returncode=result.returncode,
                       status='passed' if result.returncode == 0 else 'failed')
            save('results.json', report)
            result.check_returncode()
        report['lock_sha256'] = digest(project / 'Cargo.lock')
        report['rustc_check_passed'] = True

        helpers = runpy.run_path(str(REPO / 'scripts/measure_rust_analyzer.py'))
        report['lsp_helper_sha256'] = digest(REPO / 'scripts/measure_rust_analyzer.py')
        settings = {
            'checkOnSave': False,
            'cargo': {'allTargets': False, 'extraArgs': ['--offline', '--locked']},
            'procMacro': {'enable': True}, 'cachePriming': {'enable': True},
        }
        uri = (project / 'src/main.rs').as_uri()
        report['server_command'] = [str(server)]
        report['settings'] = settings
        report['status'] = 'observing_editor'
        save('results.json', report)
        log = (output / 'server.stderr').open('w')
        client = helpers['LspClient'](server, project, environment, log, settings)
        client.workspace_folders = [{'uri': project.as_uri(), 'name': 'minimal-native-hover'}]
        initialize_params = {
            'processId': os.getpid(), 'rootUri': project.as_uri(),
            'workspaceFolders': client.workspace_folders,
            'capabilities': {
                'workspace': {'configuration': True, 'workspaceFolders': True},
                'window': {'workDoneProgress': True},
                'experimental': {'serverStatusNotification': True},
                'textDocument': {'hover': {'contentFormat': ['markdown', 'plaintext']}},
            },
            'initializationOptions': settings,
        }
        initialized = client.request('initialize', initialize_params, 30)
        save('initialize.json', {'request': initialize_params, 'response': initialized})
        if 'error' in initialized:
            raise RuntimeError(f'initialize failed: {initialized["error"]}')
        report['server_info'] = initialized.get('result', {}).get('serverInfo')
        client.notify('initialized', {})
        client.notify('textDocument/didOpen', {'textDocument': {
            'uri': uri, 'languageId': 'rust', 'version': 1, 'text': SOURCE}})
        client.wait_ready(90)
        report['server_status'] = client.last_status
        resolved = {}
        for name in ('sum', 'annotated', 'control', 'difference'):
            params = {'textDocument': {'uri': uri},
                      'position': helpers['position'](SOURCE, 'hover-' + name, name)}
            result = client.request('textDocument/hover', params, 30)
            save('hover-' + name + '.json', {'request': params, 'response': result})
            report['hover'][name] = result
            save('results.json', report)
            if 'error' in result:
                raise RuntimeError(f'{name} hover request failed: {result["error"]}')
            # Null/unknown types are completed but invalid observations. Matching
            # the whole declaration avoids incidental N<7> in docs or examples.
            resolved[name] = f'let {name}: N<7>' in json.dumps(result.get('result'))
        report['resolved_to_n7'] = resolved
        report['valid_inference'] = all(resolved.values())
        report['status'] = 'complete'
        returncode = 0 if report['valid_inference'] else 1
        print(json.dumps({'status': report['status'], 'valid_inference': report['valid_inference'],
                          'resolved_to_n7': resolved, 'results': str(output / 'results.json')}), flush=True)
    except (Exception, KeyboardInterrupt) as error:
        report['status'] = 'interrupted' if isinstance(error, KeyboardInterrupt) else 'error'
        report['error'] = {'type': type(error).__name__, 'message': str(error)}
        print(json.dumps({'status': report['status'], 'error': report['error'],
                          'results': str(output / 'results.json')}), flush=True)
        returncode = 2
    finally:
        if client is not None:
            client.close()
            # Retain protocol observations without presenting helper timings as
            # benchmark data: other correctness jobs may run concurrently.
            save('events.json', [{key: value for key, value in event.items() if key != 'seconds'}
                                 for event in client.events])
        if log is not None:
            log.close()
        save('results.json', report)
    return returncode


if __name__ == '__main__':
    raise SystemExit(main())
