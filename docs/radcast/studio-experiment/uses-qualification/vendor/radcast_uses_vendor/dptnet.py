# The implementation of DPTNet proposed in
# J. Chen, Q. Mao, and D. Liu, “Dual-path transformer network:
# Direct context-aware modeling for end-to-end monaural speech
# separation,” in Proc. ISCA Interspeech, 2020, pp. 2642–2646.
#
# Ported from https://github.com/ujscjj/DPTNet

import torch.nn as nn
from .tcn import choose_norm
from .compat import get_activation


class ImprovedTransformerLayer(nn.Module):
    """Container module of the (improved) Transformer proposed in [1].

    Reference:
        Dual-path transformer network: Direct context-aware modeling for end-to-end
        monaural speech separation; Chen et al, Interspeech 2020.

    Args:
        rnn_type (str): select from 'RNN', 'LSTM' and 'GRU'.
        input_size (int): Dimension of the input feature.
        att_heads (int): Number of attention heads.
        hidden_size (int): Dimension of the hidden state.
        dropout (float): Dropout ratio. Default is 0.
        activation (str): activation function applied at the output of RNN.
        bidirectional (bool, optional): True for bidirectional Inter-Chunk RNN
            (Intra-Chunk is always bidirectional).
        norm (str, optional): Type of normalization to use.
    """

    def __init__(
        self,
        rnn_type,
        input_size,
        att_heads,
        hidden_size,
        dropout=0.0,
        activation="relu",
        bidirectional=True,
        norm="gLN",
    ):
        super().__init__()

        rnn_type = rnn_type.upper()
        assert rnn_type in [
            "RNN",
            "LSTM",
            "GRU",
        ], f"Only support 'RNN', 'LSTM' and 'GRU', current type: {rnn_type}"
        self.rnn_type = rnn_type

        self.att_heads = att_heads
        self.self_attn = nn.MultiheadAttention(input_size, att_heads, dropout=dropout)
        self.dropout = nn.Dropout(p=dropout)
        self.norm_attn = choose_norm(norm, input_size)

        self.rnn = getattr(nn, rnn_type)(
            input_size,
            hidden_size,
            1,
            batch_first=True,
            bidirectional=bidirectional,
        )

        hdim = 2 * hidden_size if bidirectional else hidden_size
        if activation.lower() == "linear":
            activation = nn.Identity()
        else:
            activation = get_activation(activation)
        self.feed_forward = nn.Sequential(
            activation, nn.Dropout(p=dropout), nn.Linear(hdim, input_size)
        )

        self.norm_ff = choose_norm(norm, input_size)

    def forward(self, x, attn_mask=None):
        # (batch, seq, input_size) -> (seq, batch, input_size)
        src = x.permute(1, 0, 2)
        # (seq, batch, input_size) -> (batch, seq, input_size)
        out = self.self_attn(src, src, src, attn_mask=attn_mask)[0].permute(1, 0, 2)
        out = self.dropout(out) + x
        # ... -> (batch, input_size, seq) -> ...
        out = self.norm_attn(out.transpose(-1, -2)).transpose(-1, -2)

        out2 = self.feed_forward(self.rnn(out)[0])
        out2 = self.dropout(out2) + out
        return self.norm_ff(out2.transpose(-1, -2)).transpose(-1, -2)
