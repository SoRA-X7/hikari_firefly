from pathlib import Path
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch import Tensor, IntTensor
from torch.optim.adam import Adam
from torch.utils.data import Dataset, DataLoader
import lightning as L
from tqdm import tqdm

from data_load import Replay, from_msgpack

# Constants
PIECE_KIND_EMBED_DIM = 8  # 7 kinds + 1 empty

BOARD_IN = 10 * 40  # W * H
QUEUE_IN = PIECE_KIND_EMBED_DIM * 18  # embedding * up to 18 pieces
HOLD_IN = PIECE_KIND_EMBED_DIM  # embedding
BAG_IN = PIECE_KIND_EMBED_DIM  # embedding bag
REN_IN = 1
B2B_IN = 1
CURRENT_PIECE_IN = PIECE_KIND_EMBED_DIM * 2  # current, unhold
PLACEMENT_IN = 10 * 40 + 3  # W * H + (spin kinds)

ALL_IN = (
    BOARD_IN
    + QUEUE_IN
    + HOLD_IN
    + BAG_IN
    + REN_IN
    + B2B_IN
    + CURRENT_PIECE_IN
    + PLACEMENT_IN
)


class NNEvaluator(nn.Module):
    def __init__(self):
        super(NNEvaluator, self).__init__()
        self.ft = nn.Linear(ALL_IN, 256)
        self.l1 = nn.Linear(256, 32)
        self.l2 = nn.Linear(32, 32)
        self.out = nn.Linear(32, 1)

    def forward(self, x: Tensor) -> Tensor:
        x = torch.clamp(self.ft(x))
        x = torch.clamp(self.l1(x))
        x = torch.clamp(self.l2(x))
        x = self.out(x)
        return x


class LitNNEvaluator(L.LightningModule):
    def __init__(self):
        super(LitNNEvaluator, self).__init__()
        self.model = NNEvaluator()

    def forward(self, x: Tensor) -> Tensor:
        return self.model(x)

    def training_step(self, batch: tuple[Tensor, Tensor]) -> Tensor:
        x, y = batch
        y_hat = self(x)
        loss = F.mse_loss(y_hat, y)
        self.log("train_loss", loss, on_epoch=True)
        return loss

    def configure_optimizers(self):
        return Adam(self.parameters(), lr=1e-3)


class EvalDataset(Dataset):
    def __init__(self, file_path: Path):
        self.data: list[Replay] = []
        self.tensors: list[Tensor] = []

        files = [
            path
            for path in file_path.iterdir()
            if path.is_file() and path.suffix == ".bin"
        ]
        for path in tqdm(files):
            if path.is_file():
                self.data.extend(from_msgpack(str(path)))

        for replay in self.data:
            self.tensors.append(
                torch.cat(
                    [
                        # board
                        torch.tensor(replay.state.board, dtype=torch.float32).flatten(),
                        # queue
                        torch.tensor(
                            replay.state.queue + [0] * (18 - len(replay.state.queue)),
                            dtype=torch.float32,
                        ).flatten(),
                        # hold
                        torch.tensor(
                            replay.state.hold if replay.state.hold else [0] * 7,
                            dtype=torch.float32,
                        ).flatten(),
                        # bag
                    ]
                )
            )

        print(f"Loaded {len(self.data)} replays")

    def __len__(self):
        return len(self.data)

    def __getitem__(self, idx) -> Tensor:
        return self.data[idx].into_tensor_only_board()


def convert_replay_to_tensors(replay: Replay) -> tuple[Tensor, Tensor]:
    pass
