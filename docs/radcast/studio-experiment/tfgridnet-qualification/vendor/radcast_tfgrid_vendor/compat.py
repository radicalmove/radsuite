import torch

class AbsSeparator(torch.nn.Module):
    pass

def is_complex(x):
    if not isinstance(x,torch.Tensor):raise TypeError("Native tensors required")
    return torch.is_complex(x)

def new_complex_like(reference,parts):
    if not torch.is_complex(reference):raise TypeError("Native complex reference required")
    return torch.complex(*parts)
