#!/usr/bin/env python3
"""Generate two independent workspaces, changing only the physical axis list.

Both use b45de40's actual numeral bridge and float conversion tables. The Unit
implementations preserve src/unit.rs's const-output and per-exponent obligations.
This is a bounded representation prototype, not a replacement whippyunits crate.
"""
from pathlib import Path
import argparse
import hashlib
import io
import json
import shutil
import subprocess
import tarfile

PACKAGE = Path(__file__).resolve().parent
ROOT = Path('/tmp/whippyunits-four-axis')
SOURCE = ROOT / 'source'
REVISION = 'b45de40f08f9b65277cf4a4a924f3afb7e0cd86d'
SCALE = [('P2', '_2'), ('P3', '_3'), ('P5', '_5'), ('PI', '_Pi')]
FULL = [('M', '_M'), ('L', '_L'), ('T', '_T'), ('I', '_I'),
        ('TH', '_Theta'), ('AMT', '_N'), ('J', '_J'), ('ANG', '_A')]
FOUR = [pair for pair in FULL if pair[0] in ('M', 'L', 'T', 'ANG')]


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def gen(physical):
    axes = SCALE + physical
    def params(suffixes):
        return ', '.join(f'const {a}{s}: i16' for s in suffixes for a, _ in axes)
    def unit(suffix):
        scale = ', '.join(f'{m}<{a}{suffix}>' for a, m in SCALE)
        dim = ', '.join(f'{m}<{a}{suffix}>' for a, m in physical)
        return f'Unit<Scale<{scale}>, Dimension<{dim}>>'
    code = ['use core::marker::PhantomData;',
            'use core::ops::{Add, Sub, Neg};',
            'use whippyunits_core::num::{N, NumEq, Halve, BitAll};']
    for _, marker in axes:
        code.append(f'#[derive(Clone, PartialEq)] pub struct {marker}<const EXP:i16=0>;')
    for name, parts in [('Scale', SCALE), ('Dimension', physical)]:
        generics = ', '.join(f'{a}={m}<0>' for a, m in parts)
        fields = ', '.join(a for a, _ in parts)
        code.append(f'#[derive(Clone, PartialEq)] pub struct {name}<{generics}>(PhantomData<({fields})>);')
    code.append('#[derive(Clone, PartialEq)] pub struct Unit<S,D>(PhantomData<(S,D)>);')
    for name in ['UnitMul<Rhs>', 'UnitDiv<Rhs>', 'UnitInv', 'UnitSqrt', 'UnitEq<Rhs>']:
        code.append(f'pub trait {name} {{ type Output; }}')
    for name, oper in [('UnitMul','Add'),('UnitDiv','Sub')]:
        bounds = ',\n'.join(f'N<{a}_1>: {oper}<N<{a}_2>, Output=N<{a}_O>>' for a,_ in axes)
        code.append(f'impl<{params(["_1","_2","_O"])}> {name}<{unit("_2")}> for {unit("_1")}\n'
                    f'where {bounds} {{type Output={unit("_O")};}}')
        # Quantity multiplication uses the same direct per-exponent bounds as
        # api.rs/arithmetic.rs, rather than hiding them behind a new Unit trait.
        op, method, symbol = ('Mul','mul','*') if oper=='Add' else ('Div','div','/')
        code.append(f'impl<{params(["_1","_2","_O"])}, T, R, Brand> core::ops::{op}<crate::Quantity<{unit("_2")},R,Brand>> '
                    f'for crate::Quantity<{unit("_1")},T,Brand> where {bounds}, T:core::ops::{op}<R> {{'
                    f'type Output=crate::Quantity<{unit("_O")},<T as core::ops::{op}<R>>::Output,Brand>;'
                    f'fn {method}(self,rhs:crate::Quantity<{unit("_2")},R,Brand>)->Self::Output{{'
                    f'crate::Quantity::new(self.raw {symbol} rhs.raw)}} }}')
    for name, oper in [('UnitInv','Neg'),('UnitSqrt','Halve')]:
        bounds = ',\n'.join(f'N<{a}>:{oper}<Output=N<{a}_O>>' for a,_ in axes)
        code.append(f'impl<{params(["","_O"])}> {name} for {unit("")} where {bounds} '
                    f'{{type Output={unit("_O")};}}')
    bits = '()'
    for a,_ in reversed(axes):
        bits = f'(<N<{a}_1> as NumEq<N<{a}_2>>>::Output, {bits})'
    bounds = ',\n'.join(f'N<{a}_1>:NumEq<N<{a}_2>>' for a,_ in axes)
    code.append(f'impl<{params(["_1","_2"])}> UnitEq<{unit("_2")}> for {unit("_1")} '
                f'where {bounds}, {bits}:BitAll {{type Output=<{bits} as BitAll>::Output;}}')
    code += ['pub type MulUnit<A,B>=<A as UnitMul<B>>::Output;',
             'pub type DivUnit<A,B>=<A as UnitDiv<B>>::Output;',
             'pub type InvUnit<U>=<U as UnitInv>::Output;',
             'pub type SqrtUnit<U>=<U as UnitSqrt>::Output;']
    # The public alias is convenient spelling only: Dimension itself really has
    # four fields/type parameters in the candidate, and eight in the baseline.
    ds = ','.join(f'{marker}<{axis if axis in ("M","L","T","ANG") else "0"}>' for axis,marker in physical)
    code.append('pub type U<const P2:i16,const P3:i16,const P5:i16,const PI:i16,'
                'const M:i16,const L:i16,const T:i16,const ANG:i16>='
                f'Unit<Scale<_2<P2>,_3<P3>,_5<P5>,_Pi<PI>>,Dimension<{ds}>>;')
    code.append('pub const PHYSICAL_AXES:usize='+str(len(physical))+';')
    return '\n'.join(code)+'\n'


def prepare_snapshot(repo):
    if not SOURCE.exists():
        try:
            blob = subprocess.check_output(['git','-C',str(repo),'archive',REVISION])
        except subprocess.CalledProcessError as error:
            raise SystemExit(f'Cannot archive baseline {REVISION} from {repo}; '
                             'use --repo with a checkout containing that commit.') from error
        SOURCE.mkdir(parents=True)
        with tarfile.open(fileobj=io.BytesIO(blob)) as archive:
            archive.extractall(SOURCE, filter='data')
    expected=json.loads((PACKAGE/'source-provenance.json').read_text())
    assert expected['commit']==REVISION
    for name,digest in expected['sha256'].items():
        if hashlib.sha256((SOURCE/name).read_bytes()).hexdigest()!=digest:
            raise SystemExit(f'Baseline input changed: {SOURCE/name}. Use a fresh --work-dir.')


def main():
    global ROOT,SOURCE
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work-dir',type=Path,default=ROOT)
    parser.add_argument('--repo',type=Path,default=PACKAGE.parents[2])
    args=parser.parse_args()
    ROOT=args.work_dir.resolve()
    SOURCE=ROOT/'source'
    ROOT.mkdir(parents=True,exist_ok=True)
    prepare_snapshot(args.repo)
    (ROOT/'common').mkdir(exist_ok=True)
    for name in ['engine.rs','consumer.rs','safety.rs']:
        shutil.copyfile(PACKAGE/'common'/name,ROOT/'common'/name)
    # Reuse the identical float conversion implementation from b45de40.
    table = (SOURCE/'src/scale_conversion.rs').read_text()
    table = table[table.index('const fn float_powers'):]
    # Stop after pow_pi_float; following macro comments are irrelevant.
    end = table.index('\n}', table.index('pub const fn pow_pi_float'))+2
    write(ROOT/'common/float_tables.rs', table[:end]+'\n')
    # Exact current dimension-list recursion, without the parser-facing macro
    # or the concrete 8-axis rescale impl (the prototype supplies its own).
    dims = (SOURCE/'whippyalgebra/src/dims.rs').read_text()
    dims = dims[:dims.index('// Rescaling: element-wise')]
    dims = dims[dims.index('/// The empty dimension list.'):]
    dims = dims.replace('whippyunits::unit!(1)', 'crate::One')
    dims = 'use core::marker::PhantomData;\nuse crate::{Unit, UnitMul, UnitDiv, UnitInv, MulUnit};\n'+dims
    write(ROOT/'common/dims.rs', dims)
    for module in ['index', 'uniformity']:
        code = (SOURCE/f'whippyalgebra/src/{module}.rs').read_text().replace('use whippyunits::', 'use crate::')
        write(ROOT/f'common/{module}.rs', code)
    for name, physical in [('eight', FULL), ('four', FOUR)]:
        root=ROOT/name
        write(root/'Cargo.toml','[workspace]\nresolver="2"\nmembers=["engine","consumer"]\n')
        write(root/'rust-toolchain.toml','[toolchain]\nchannel="1.98.1"\nprofile="minimal"\n')
        shutil.copyfile(PACKAGE/'Cargo.lock.fixture',root/'Cargo.lock')
        write(root/'engine/Cargo.toml','''[package]
name="axis-engine"
version="0.0.0"
edition="2024"
[lib]
doctest=false
[features]
default=["nalgebra"]
nalgebra=[]
[dependencies]
whippyunits-core={path="../../source/whippyunits-core",default-features=false}
typenum={version="=1.20.1",features=["const-generics"]}
nalgebra={version="=0.34.2",default-features=false,features=["libm"]}
''')
        write(root/'consumer/Cargo.toml','''[package]
name="axis-consumer"
version="0.0.0"
edition="2024"
[lib]
doctest=false
[features]
stress=[]
[dependencies]
axis-engine={path="../engine"}
typenum="=1.20.1"
nalgebra={version="=0.34.2",default-features=false,features=["libm"]}
''')
        write(root/'engine/src/representation.rs',gen(physical))
        write(root/'engine/src/lib.rs','''#![allow(non_camel_case_types)]
mod representation;
pub use representation::*;
'''+(ROOT/'common/engine.rs').read_text()+'''
pub mod float_tables;
pub mod dims;
pub mod index;
pub mod uniformity;
''')
        # Keep every Rust source inside its Cargo crate. Cross-workspace include!
        # paths compile with rustc but may fall outside rust-analyzer's source
        # roots, leaving Quantity and its consumers unresolved in the LSP probe.
        for module in ['float_tables','dims','index','uniformity']:
            write(root/f'engine/src/{module}.rs',(ROOT/f'common/{module}.rs').read_text())
        write(root/'consumer/src/lib.rs','#[cfg(feature="stress")] pub mod stress;\n'+(ROOT/'common/consumer.rs').read_text())
        write(root/'consumer/src/main.rs',(PACKAGE/'common/ra_main.rs').read_text())
        write(root/'consumer/tests/safety.rs',(ROOT/'common/safety.rs').read_text())
        # Identical feature-gated body checks for many distinct low-exponent
        # mixed-scale instantiations, without substituting catalog generation.
        stress=[]
        for n in range(96):
            s2=(n%5)-2; s3=((n//5)%3)-1; s5=((n//15)%3)-1; ang=(n//45)%2
            length=1+(n//90); time=(n//30)%3-1
            stress.append(f'''pub fn case_{n}(x:f64)->f64 {{
type X=U<{s2},{s3},{s5},0,0,{length},{time},{ang}>;
let value=Quantity::<X>::new(x);
let dt=Quantity::<Tick>::new(2.0);
let velocity=value/dt;
let accel=velocity/dt;
let force=accel*Quantity::<Kilogram>::new(3.0);
let energy=force*value;
let back=energy/value/Quantity::<Kilogram>::new(3.0)*dt*dt;
let restored:Quantity<X>=back;
restored.raw
}}''')
        write(root/'consumer/src/stress.rs','use axis_engine::*;\n'+'\n'.join(stress)+'\n')
    write(ROOT/'provenance.json',(PACKAGE/'source-provenance.json').read_text())
    print(f'Generated eight/four-axis fixtures in {ROOT}; baseline {REVISION}')


if __name__=='__main__': main()
