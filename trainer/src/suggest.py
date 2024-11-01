from pathlib import Path
import lightning as L
from torch import nn, Tensor
import torch
import torch.nn.functional as F
from torch.utils.data import DataLoader, Dataset, random_split
from tqdm import tqdm


from data_load import Replay, from_msgpack


def _piece_kind_to_ints(kind: str) -> list[int]:
    if kind == "I":
        return [1, 0, 0, 0, 0, 0, 0]
    if kind == "O":
        return [0, 1, 0, 0, 0, 0, 0]
    if kind == "T":
        return [0, 0, 1, 0, 0, 0, 0]
    if kind == "S":
        return [0, 0, 0, 1, 0, 0, 0]
    if kind == "Z":
        return [0, 0, 0, 0, 1, 0, 0]
    if kind == "J":
        return [0, 0, 0, 0, 0, 1, 0]
    if kind == "L":
        return [0, 0, 0, 0, 0, 0, 1]
    raise ValueError(f"Invalid piece kind: {kind}")


class BoardModule(nn.Module):
    IN = 10 * 40  # W * H
    OUT = 512

    def __init__(self):
        super(BoardModule, self).__init__()
        self.conv1 = nn.Conv2d(1, 32, 3)  # 8x38
        self.conv2 = nn.Conv2d(32, 256, 3)  # 6x36
        self.conv3 = nn.Conv2d(256, 256, 3)  # 4x34
        self.fc1 = nn.Linear(256 * 4 * 34, BoardModule.OUT)

    def forward(self, x: Tensor) -> Tensor:
        x = F.relu(self.conv1(x))
        x = F.relu(self.conv2(x))
        x = F.relu(self.conv3(x))
        x = x.flatten(1)
        x = F.relu(self.fc1(x))
        return x


class SuggesterModel(nn.Module):
    IN = BoardModule.OUT + 19 * 8 + 1 + 1  # board + queue + hold + ren + b2b
    OUT = 7 * 14 * 24 + 2

    def __init__(self):
        super(SuggesterModel, self).__init__()
        self.ft = nn.Linear(BoardModule.OUT + 19 * 8 + 1 + 1, 256)
        self.l1 = nn.Linear(256, 256)
        self.l2 = nn.Linear(256, 256)
        self.out = nn.Linear(256, SuggesterModel.OUT)

    def forward(self, board: Tensor, meta: Tensor) -> Tensor:
        x = torch.clamp(self.ft(x))
        x = torch.clamp(self.l1(x))
        x = torch.clamp(self.l2(x))
        x = self.out(x)
        return x


class LitModule(L.LightningModule):
    def __init__(self):
        super(LitModule, self).__init__()
        self.board = BoardModule()
        self.model = SuggesterModel()

    def forward(self, x: Tensor) -> Tensor:
        board, rem = x[:, :400], x[:, 400:]
        board = self.board(board)
        x = self.model(board, rem)
        return x

    def training_step(self, batch: tuple[Tensor, Tensor], batch_idx: int):
        x, y = batch
        y_hat = self(x)
        loss = F.mse_loss(y_hat, y)
        self.log("train_loss", loss)
        return loss

    def validation_step(self, batch: tuple[Tensor, Tensor], batch_idx: int):
        x, y = batch
        y_hat = self(x)
        loss = F.mse_loss(y_hat, y)
        self.log("val_loss", loss)
        return loss

    def configure_optimizers(self):
        return torch.optim.adam.Adam(self.parameters(), lr=1e-3)


class SuggestDataset(Dataset):
    def __init__(self, file_path: Path):
        self.data: list[Replay] = []
        self.inputs: list[Tensor] = []
        self.outputs: list[Tensor] = []

        files = [
            path
            for path in file_path.iterdir()
            if path.is_file() and path.suffix == ".bin"
        ]
        for path in tqdm(files):
            if path.is_file():
                self.data.extend(from_msgpack(str(path)))

        for replay in self.data:
            self.inputs.append(
                torch.cat(
                    [
                        # board
                        torch.tensor(
                            replay.state.board[0:40], dtype=torch.float32
                        ).flatten(),
                        # queue
                        torch.tensor(
                            [_piece_kind_to_ints(q) for q in replay.state.queue]
                            + [0] * (18 - len(replay.state.queue)),
                            dtype=torch.float32,
                        ).flatten(),
                        # hold
                        torch.tensor(
                            (
                                _piece_kind_to_ints(replay.state.hold)
                                if replay.state.hold
                                else [0] * 7
                            ),
                            dtype=torch.float32,
                        ).flatten(),
                        # ren
                        torch.tensor([replay.state.ren], dtype=torch.float32) / 20.0,
                        # b2b
                        torch.tensor(
                            [1 if replay.state.b2b else 0], dtype=torch.float32
                        ),
                    ]
                )
            )
            self.outputs.append()

        print(f"Loaded {len(self.data)} replays")

    def __len__(self):
        return len(self.data)

    def __getitem__(self, idx) -> tuple[Tensor, Tensor]:
        return self.data[idx].into_tensor_only_board()


def train():
    dataset = SuggestDataset(Path("../data"))

    # use 20% of training data for validation
    train_set_size = int(len(dataset) * 0.8)
    valid_set_size = len(dataset) - train_set_size
    # split the train set into two
    seed = torch.Generator().manual_seed(42)
    train_set, valid_set = random_split(
        dataset, [train_set_size, valid_set_size], generator=seed
    )

    train_loader = DataLoader(train_set, batch_size=64, shuffle=True)
    valid_loader = DataLoader(valid_set, batch_size=64)

    model = LitModule()
    trainer = L.Trainer(max_epochs=10)
    trainer.fit(model, train_loader, valid_loader)


if __name__ == "__main__":
    train()
