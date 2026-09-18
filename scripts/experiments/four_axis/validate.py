#!/usr/bin/env python3
"""Run identical runtime tests and external compile-pass/fail fixtures for both."""
import argparse
import json
import os
from pathlib import Path
import subprocess

ROOT=Path('/tmp/whippyunits-four-axis')
PROBES={
 'pass': (True, '''use axis_engine::*;
fn main() {
 let _:Quantity<Second>=Quantity::<Tick>::new(60.0).rescale();
 let _:Quantity<Radian>=Quantity::<Degree>::new(180.0).rescale();
 let x=Quantity::<Meter>::new(2.0);
 let t=Quantity::<Second>::new(3.0);
 let _:Quantity<Force>=Quantity::<Kilogram>::new(4.0)*x/t/t;
 let _:Quantity<Meter>=(x*x).sqrt();
}'''),
 'incompatible_add': (False, '''use axis_engine::*;
fn main(){let _=Quantity::<Meter>::new(1.0)+Quantity::<Second>::new(1.0);}'''),
 'incompatible_rescale': (False, '''use axis_engine::*;
fn main(){let _:Quantity<Second>=Quantity::<Meter>::new(1.0).rescale();}'''),
 'angle_is_not_scalar': (False, '''use axis_engine::*;
fn main(){let _:Quantity<One>=Quantity::<Degree>::new(1.0).rescale();}'''),
 'scale_is_not_erased': (False, '''use axis_engine::*;
fn main(){let _:Quantity<Second>=Quantity::<Tick>::new(1.0);}'''),
 'sqrt_odd_axis': (False, '''use axis_engine::*;
fn main(){let _=Quantity::<Meter>::new(1.0).sqrt();}'''),
 'wrong_matrix_inner_units': (False, '''use axis_engine::*;
use axis_engine::dims::{DCons,DNil};
type State=DCons<Meter,DCons<Velocity,DNil>>;
type Wrong=DCons<Second,DCons<Velocity,DNil>>;
fn multiply<A:core::ops::Mul<B>,B>() {}
fn main(){multiply::<Mixed<State,State,2,2>,Mixed<Wrong,State,2,2>>();}'''),
}


def main():
    global ROOT
    parser=argparse.ArgumentParser()
    parser.add_argument('--work-dir',type=Path,default=ROOT)
    parser.add_argument('--target-root',type=Path)
    args=parser.parse_args()
    ROOT=args.work_dir.resolve()
    args.target_root=args.target_root or ROOT/'validation-target'
    results=[]
    for variant in ['eight','four']:
        target=(args.target_root/variant).resolve()
        env=dict(os.environ,CARGO_TARGET_DIR=str(target),CARGO_INCREMENTAL='0',RUSTUP_TOOLCHAIN='1.98.1')
        cmd=['cargo','test','--offline','--locked','--manifest-path',str(ROOT/variant/'Cargo.toml'),
             '-p','axis-consumer','--features','stress','--message-format=json']
        build=subprocess.run(cmd,env=env,text=True,capture_output=True)
        (ROOT/f'validation-{variant}.log').write_text(build.stdout+'\n'+build.stderr)
        if build.returncode:
            raise SystemExit(f'{variant} tests failed: see validation-{variant}.log\n'+build.stderr[-4000:])
        artifact=None
        for line in build.stdout.splitlines():
            try: event=json.loads(line)
            except json.JSONDecodeError: continue
            if event.get('reason')=='compiler-artifact' and event.get('target',{}).get('name')=='axis_engine':
                artifact=next((p for p in event['filenames'] if p.endswith('.rlib')),artifact)
        assert artifact, 'Cargo did not report axis_engine rlib'
        probes=ROOT/'probes'/variant
        probes.mkdir(parents=True,exist_ok=True)
        for name,(expected,source) in PROBES.items():
            path=probes/f'{name}.rs'
            path.write_text(source+'\n')
            cmd=['rustc','--edition=2024',str(path),'--emit=metadata','--out-dir',str(probes),
                 '--extern',f'axis_engine={artifact}','-L',f'dependency={target}/debug/deps']
            proc=subprocess.run(cmd,env=env,text=True,capture_output=True)
            (probes/f'{name}.stderr').write_text(proc.stderr)
            passed=proc.returncode==0
            assert passed==expected, f'{variant}/{name} unexpectedly returned {proc.returncode}: {proc.stderr}'
            if not passed:
                assert any(f'error[{code}]' in proc.stderr for code in ['E0308','E0271','E0277','E0599']), proc.stderr
            results.append({'variant':variant,'probe':name,'expected_compile_pass':expected,
                            'actual_compile_pass':passed})
        print(f'{variant}: 5 runtime tests, 96 stress functions, 1 compile-pass + 6 compile-fail probes passed',flush=True)
    (ROOT/'validation.json').write_text(json.dumps(results,indent=2)+'\n')


if __name__=='__main__': main()
