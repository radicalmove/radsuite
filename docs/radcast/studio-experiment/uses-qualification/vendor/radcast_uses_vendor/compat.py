"""Native torch-complex compatibility only; unsupported legacy branches fail."""
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
