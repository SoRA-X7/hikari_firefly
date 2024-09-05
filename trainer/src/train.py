from pathlib import Path
from pydantic import BaseModel
import torch
from torch import nn
import torch.nn.functional as F
import torch.utils.data as data
from torch.utils.data import DataLoader, Dataset
import lightning as L
from data_load import Replay, from_msgpack
from tqdm import tqdm


class EvaluatorModel(nn.Module):
    W = 10
    H = 64

    def __init__(self):
        super().__init__()
        # 1x10x64
        self.conv_0 = nn.Conv2d(1, 16, 3)
        # 16x8x62
        self.conv_1 = nn.Conv2d(16, 32, 3)
        # 32x6x60
        self.pool = nn.MaxPool2d(2)
        # 32x3x30
        self.l_1 = nn.Linear(32 * 3 * 30 + 156, 1024)
        self.l_2 = nn.Linear(1024, 10 * 64 + 2)

    def forward(self, board: torch.Tensor, meta: torch.Tensor) -> torch.Tensor:
        board = F.relu(self.conv_0(board))
        # print(1, board.shape)
        board = self.pool(F.relu(self.conv_1(board)))
        # print(2, board.shape, board.flatten().shape, meta.shape)
        x = torch.cat([board.flatten(), meta.flatten()]).unsqueeze(0)
        # print(3, x.shape)
        x = F.relu(self.l_1(x))
        x = F.relu(self.l_2(x))
        return x


class LitEvaluator(L.LightningModule):
    def __init__(self):
        super().__init__()
        self.model = EvaluatorModel()

    def training_step(self, batch: list[torch.Tensor], batch_idx):
        board, meta, eval = batch
        eval_hat = self.model(board, meta)
        # print(eval_hat)
        loss = F.binary_cross_entropy(eval_hat, eval)
        return loss

    def validation_step(self, batch: list[torch.Tensor], batch_idx):
        board, meta, eval = batch
        eval_hat = self.model(board, meta)
        loss = F.binary_cross_entropy(eval_hat, eval)
        self.log("test_loss", loss)
        return loss

    def configure_optimizers(self):
        return torch.optim.Adam(self.parameters(), lr=0.00001)  # type: ignore


class StackerDataset(Dataset):
    def __init__(self, file_path: Path):
        self.data: list[Replay] = []
        files = [
            path
            for path in file_path.iterdir()
            if path.is_file() and path.suffix == ".bin"
        ]
        for path in tqdm(files):
            if path.is_file():
                self.data.extend(from_msgpack(str(path)))

        print(f"Loaded {len(self.data)} replays")

    def __len__(self):
        return len(self.data)

    def __getitem__(self, idx) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        return self.data[idx].into_tensor()


def main():
    dataset = StackerDataset(Path("../data"))
    b, m, e = dataset[881]
    print(b.shape, m.shape, e.shape)

    # use 20% of training data for validation
    train_set_size = int(len(dataset) * 0.8)
    valid_set_size = len(dataset) - train_set_size

    # split the train set into two
    seed = torch.Generator().manual_seed(42)
    train_set, valid_set = data.random_split(
        dataset, [train_set_size, valid_set_size], generator=seed
    )
    print(len(train_set), len(valid_set))

    train_data_loader = DataLoader(train_set, num_workers=1)
    valid_data_loader = DataLoader(valid_set, num_workers=1)

    model = LitEvaluator()

    trainer = L.Trainer(accelerator="mps", max_epochs=10, min_epochs=3)
    trainer.fit(model, train_data_loader, valid_data_loader)


if __name__ == "__main__":
    main()
