"""Extract the pinned, audited legacy USES inference subset without ESPnet installs.

Original classes/functions are copied verbatim; only module imports change.
This builder never loads a checkpoint or runs enhancement.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import shutil

HERE=Path(__file__).resolve().parent
CODE_REVISION='bfc13cecfd0a07ed8e21d733b0ce130a1c69211a'
MODEL_REVISION='927a9ecea245120a6f2d88c2552864b937ec5ab9'
PINNED={
    'uses.py':'cf666616693d3f2ed6d804780b2e6e768dcd8545f5de933d695f7918cce8e6e3',
    'uses_separator.py':'be01dd72c1a92be733d7a8f1a6fa3f2163561a2500266b96d061d9640714bc69',
    'dptnet.py':'16aa8e9a09b955aa425b586d43cc26e1203ffc16837b34ee8cbe42e0b7dc2e17',
    'tcn.py':'c2dd523ebf5e34b35017207dacc8c23e1b0bc3b399c8bf360f66c4fb3cc9a9cd',
    'get_layer_from_string.py':'f8821d9566f326f75e69339eca18fa3b84d81b84915502dddbfe64b30dc0884e',
    'LICENSE-espnet':'4696c3c9551da6fef1368be1e4ed2c80cf13e55448c6dcf2aba9462f5ff29ef5',
}
SELECTION={
    'uses.py':('USES','ATFBlock','ChannelAttention','ChannelTAC','LayerNormalization'),
    'uses_separator.py':('USESSeparator',),
    'dptnet.py':('ImprovedTransformerLayer',),
    'tcn.py':('choose_norm','ChannelwiseLayerNorm','GlobalLayerNorm'),
    'get_layer_from_string.py':('get_layer',),
}
HEADERS={
    'uses.py':'''import warnings
import torch
import torch.nn as nn
from .dptnet import ImprovedTransformerLayer as SingleTransformer
from .tcn import ChannelwiseLayerNorm
from .get_layer_from_string import get_layer
''',
    'uses_separator.py':'''from collections import OrderedDict
from typing import Dict, List, Optional, Tuple, Union
import torch
from .compat import AbsSeparator, ComplexTensor, is_complex, new_complex_like
from .uses import USES
''',
    'dptnet.py':'''import torch.nn as nn
from .tcn import choose_norm
from .compat import get_activation
''',
    'tcn.py':'''import torch
import torch.nn as nn
EPS = torch.finfo(torch.get_default_dtype()).eps
''',
    'get_layer_from_string.py':'''import difflib
import torch
''',
}
COMPAT='''"""Native torch-complex compatibility only; unsupported legacy branches fail."""
import torch


class AbsSeparator(torch.nn.Module):
    """Parameter-free nn.Module base for the copied USESSeparator."""
    pass


class ComplexTensor:
    def __new__(cls, *args, **kwargs):
        raise NotImplementedError("Legacy torch_complex is unsupported; use native complex tensors")


def is_complex(value):
    if not isinstance(value, torch.Tensor):
        raise TypeError("Only native torch tensors are supported")
    return torch.is_complex(value)


def new_complex_like(reference, parts):
    if not isinstance(reference, torch.Tensor) or not torch.is_complex(reference):
        raise TypeError("A native torch complex reference is required")
    real, imaginary = parts
    return torch.complex(real, imaginary)


def get_activation(name):
    raise NotImplementedError(
        "Legacy ESPnet get_activation is unsupported; the pinned USES Transformer uses linear activation"
    )
'''


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def ast_hash(node):
    return hashlib.sha256(ast.dump(node,include_attributes=False).encode()).hexdigest()


def build(artifacts,output):
    artifacts=Path(artifacts);output=Path(output)
    for name,expected in PINNED.items():
        if sha(artifacts/name)!=expected:
            raise ValueError(f'{name} source hash mismatch')
    if output.exists():raise FileExistsError(f'Fresh vendor directory required: {output}')
    # Construct and compare all syntax trees before writing a single package file.
    modules={};symbols=[]
    for filename,names in SELECTION.items():
        source=(artifacts/filename).read_text();original=ast.parse(source)
        nodes={n.name:n for n in original.body if isinstance(n,(ast.ClassDef,ast.FunctionDef))}
        # Keep upstream pre-import attribution comments, including DPTNet's port credit.
        credit=''.join(source.splitlines(keepends=True)[:original.body[0].lineno-1])
        generated=credit+HEADERS[filename]+'\n\n'+'\n\n\n'.join(ast.get_source_segment(source,nodes[name]) for name in names)+'\n'
        copied={n.name:n for n in ast.parse(generated).body if isinstance(n,(ast.ClassDef,ast.FunctionDef))}
        for name in names:
            before=ast_hash(nodes[name]);after=ast_hash(copied[name])
            if before!=after:raise ValueError(f'{filename}:{name} AST changed')
            symbols.append(dict(name=name,source_file=filename,vendor_file=filename,ast_sha256=before))
        modules[filename]=generated
    package=output/'radcast_uses_vendor';package.mkdir(parents=True)
    for name,source in modules.items():(package/name).write_text(source)
    (package/'compat.py').write_text(COMPAT)
    (package/'__init__.py').write_text('"""Pinned legacy USES inference subset; not USES2."""\nfrom .uses_separator import USESSeparator\n')
    shutil.copy2(artifacts/'LICENSE-espnet',package/'LICENSE')
    (package/'README.md').write_text(
        '# Legacy USES inference subset\n\n'
        'Copied from ESPnet, Copyright ESPnet developers, under Apache License 2.0; '
        'the complete license is included in LICENSE. DPTNet retains its upstream attribution in the copied class.\n\n'
        f'ESPnet code revision: {CODE_REVISION}. Official checkpoint revision: {MODEL_REVISION}.\n\n'
        'This package preserves every selected class/function AST. Import bindings are local; '
        'the base separator is a parameter-free torch.nn.Module, native torch complex helpers '
        'are provided, and unsupported torch_complex/legacy activation branches fail explicitly. '
        'The pinned Transformer uses linear activation and TAC channel modeling. '
        'Unused full ESPnet code, training, inference CLI and non-native complex dependencies are omitted.\n\n'
        'The available official weights are legacy USES (ASRU 2023), not USES2. '
        'This subset does not establish any USES2 checkpoint provenance or listener quality.\n\n'
        'Source: https://github.com/espnet/espnet/tree/'+CODE_REVISION+'/espnet2/enh\n'
    )
    audit=dict(schema_version=1,model_family='legacy USES (ASRU 2023)',is_uses2_checkpoint=False,
        code_revision=CODE_REVISION,model_revision=MODEL_REVISION,source_hashes=PINNED,symbols=symbols,
        constants={'tcn.EPS':'torch.finfo(torch.get_default_dtype()).eps'},
        compatibility=dict(native_complex_only=True,legacy_get_activation='explicitly unsupported; pinned linear branch only',
                           abs_separator='parameter-free torch.nn.Module'),
        files={str(p.relative_to(output)):sha(p) for p in sorted(package.iterdir()) if p.is_file()})
    (output/'vendor-audit.json').write_text(json.dumps(audit,indent=2)+'\n')
    return audit


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--artifacts',type=Path,default=HERE/'artifacts')
    parser.add_argument('--output',type=Path,default=HERE/'vendor');args=parser.parse_args()
    audit=build(args.artifacts,args.output)
    print(json.dumps(dict(symbol_count=len(audit['symbols']),vendor_files=len(audit['files']),inference=False)))
