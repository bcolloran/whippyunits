#!/usr/bin/env python3
"""Alternate representations at one fixed fixture path and one shared target.

Dependencies are warmed first. Only representation.rs changes between variants.
GNU time reports maximum individual-process RSS (including waited-for children),
not the sum of RSS across simultaneously running Cargo/rustc processes.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import time

DEFAULT_ROOT=Path('/tmp/whippyunits-four-axis')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work-dir',type=Path,default=DEFAULT_ROOT)
    parser.add_argument('--repeats',type=int,default=7)
    parser.add_argument('--scope',choices=['engine-consumer','consumer','both'],default='both')
    parser.add_argument('--stress',action='store_true')
    parser.add_argument('--target-dir',type=Path)
    parser.add_argument('--output',type=Path)
    parser.add_argument('--gnu-time',type=Path,default=Path('/usr/bin/time'))
    args=parser.parse_args()
    if args.repeats<1: parser.error('--repeats must be positive')
    root=args.work_dir.resolve()
    target=(args.target_dir or root/'benchmark-target').resolve()
    output=(args.output or root/'benchmark.json').resolve()
    active=root/'active'
    logs=root/'benchmark-logs'/f'{args.scope}-{"stress" if args.stress else "small"}'
    logs.mkdir(parents=True,exist_ok=True)
    # Identical manifests/names/source locations prevent path/crate-disambiguator
    # and separate dependency caches from becoming accidental A/B variables.
    def rust_sources(variant):
        return {str(file.relative_to(root/variant))
                for folder in ['engine/src','consumer/src','consumer/tests']
                for file in (root/variant/folder).rglob('*.rs')
                if file.name!='representation.rs'}
    sources=rust_sources('eight')
    if sources!=rust_sources('four'):
        raise SystemExit('Fixture source sets differ. Regenerate in a fresh --work-dir.')
    shared=['Cargo.toml','Cargo.lock','rust-toolchain.toml','engine/Cargo.toml','consumer/Cargo.toml',
            *sorted(sources)]
    for relative in shared:
        baseline=root/'eight'/relative
        if baseline.read_bytes()!=(root/'four'/relative).read_bytes():
            raise SystemExit(f'Common fixture source differs: {relative}.')
        # active/ is a generated benchmark copy. Refresh its common files before
        # warmup when preparation changes layout; only representation.rs varies
        # within the actual A/B sample loop.
        destination=active/relative
        destination.parent.mkdir(parents=True,exist_ok=True)
        if not destination.exists() or destination.read_bytes()!=baseline.read_bytes():
            shutil.copyfile(baseline,destination)
    version=subprocess.check_output([str(args.gnu_time),'--version'],text=True)
    if 'GNU Time' not in version and 'GNU time' not in version:
        raise SystemExit('--gnu-time must point to GNU time (Linux %M RSS measurement).')
    env=dict(os.environ,CARGO_TARGET_DIR=str(target),CARGO_INCREMENTAL='0',RUSTUP_TOOLCHAIN='1.98.1')
    manifest=str(active/'Cargo.toml')
    def command(package='axis-consumer'):
        cmd=['cargo','check','--offline','--locked','--manifest-path',manifest,'-p',package,'--lib']
        if args.stress and package=='axis-consumer': cmd.extend(['--features','stress'])
        return cmd
    def install(variant):
        shutil.copyfile(root/variant/'engine/src/representation.rs',active/'engine/src/representation.rs')
    def clean(scope):
        cmd=['cargo','clean','--manifest-path',manifest,'-p','axis-consumer']
        if scope=='engine-consumer': cmd += ['-p','axis-engine']
        subprocess.run(cmd,env=env,cwd=active,check=True,capture_output=True)
    # Warm common dependencies and both source representations outside samples.
    for variant in ['eight','four']:
        install(variant)
        subprocess.run(command(),env=env,cwd=active,check=True)
    samples=[]
    scopes=['engine-consumer','consumer'] if args.scope=='both' else [args.scope]
    for rep in range(args.repeats):
        for variant in (['eight','four'] if rep%2==0 else ['four','eight']):
            install(variant)
            if args.scope=='consumer':
                # The correct representation is already checked for a strictly
                # consumer-only sample; rebuilding it is excluded from timing.
                subprocess.run(command('axis-engine'),env=env,cwd=active,check=True,capture_output=True)
            for scope in scopes:
                # With --scope both the successful engine+consumer check leaves
                # the correct engine cached for the immediately following sample.
                clean(scope)
                stem=logs/f'{rep:02}-{variant}-{scope}'
                rss_path=stem.with_suffix('.rss-kib')
                timed=[str(args.gnu_time),'--format=%M','--output',str(rss_path),*command()]
                start=time.perf_counter()
                proc=subprocess.run(timed,env=env,cwd=active,capture_output=True,text=True)
                elapsed=time.perf_counter()-start
                stem.with_suffix('.stdout').write_text(proc.stdout)
                stem.with_suffix('.stderr').write_text(proc.stderr)
                if proc.returncode: raise SystemExit(proc.stderr)
                peak_rss_kib=int(rss_path.read_text().strip())
                sample={'rep':rep,'variant':variant,'scope':scope,'seconds':elapsed,
                        'peak_single_process_rss_kib':peak_rss_kib}
                samples.append(sample)
                print(json.dumps(sample),flush=True)
    summaries={}
    for scope in scopes:
        selected=[s for s in samples if s['scope']==scope]
        medians={v:statistics.median(s['seconds'] for s in selected if s['variant']==v) for v in ['eight','four']}
        rss_medians={v:statistics.median(s['peak_single_process_rss_kib'] for s in selected if s['variant']==v)
                     for v in ['eight','four']}
        summaries[scope]={'medians_seconds':medians,'medians_peak_single_process_rss_kib':rss_medians,
                          'candidate_reduction_percent':100*(1-medians['four']/medians['eight'])}
    result={'scope':args.scope,'stress':args.stress,'incremental':False,'dependencies':'warm',
            'fixed_fixture_path':str(active),'shared_target':str(target),'toolchain':'1.98.1',
            'source_layout':'ordinary Rust files inside each Cargo crate; no cross-workspace include!',
            'base_commit':'b45de40f08f9b65277cf4a4a924f3afb7e0cd86d',
            'rustc':subprocess.check_output(['rustc','-Vv'],env=env,cwd=active,text=True),
            'inherited_environment':{key:env.get(key) for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS',
                                      'RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_JOBS']},
            'rss_method':'GNU time %M maximum process RSS, Linux KiB; not concurrent process-tree sum',
            'samples':samples,'summaries':summaries}
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))


if __name__=='__main__': main()
