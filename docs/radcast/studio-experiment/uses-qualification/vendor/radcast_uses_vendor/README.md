# Legacy USES inference subset

Copied from ESPnet, Copyright ESPnet developers, under Apache License 2.0; the complete license is included in LICENSE. DPTNet retains its upstream attribution in the copied class.

ESPnet code revision: bfc13cecfd0a07ed8e21d733b0ce130a1c69211a. Official checkpoint revision: 927a9ecea245120a6f2d88c2552864b937ec5ab9.

This package preserves every selected class/function AST. Import bindings are local; the base separator is a parameter-free torch.nn.Module, native torch complex helpers are provided, and unsupported torch_complex/legacy activation branches fail explicitly. The pinned Transformer uses linear activation and TAC channel modeling. Unused full ESPnet code, training, inference CLI and non-native complex dependencies are omitted.

The available official weights are legacy USES (ASRU 2023), not USES2. This subset does not establish any USES2 checkpoint provenance or listener quality.

Source: https://github.com/espnet/espnet/tree/bfc13cecfd0a07ed8e21d733b0ce130a1c69211a/espnet2/enh
