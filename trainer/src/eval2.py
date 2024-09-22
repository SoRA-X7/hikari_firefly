from pathlib import Path
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch import Tensor, IntTensor
from torch.optim.adam import Adam
from torch.utils.data import Dataset, DataLoader
import lightning as L
from tqdm import tqdm


class BoardModule(nn.Module):
    def __init__(self):
        super(BoardModule, self).__init__()
        self.conv1 = nn.Conv2d(1, 32, 3)
        self.conv2 = nn.Conv2d(32, 64, 3)
        self.fc1 = nn.Linear(64 * 6 * 6, 128)

    def forward(self, x: Tensor) -> Tensor:
        x = F.relu(self.conv1(x))
        x = F.relu(self.conv2(x))
        x = x.view(-1, 64 * 6 * 6)
        x = F.relu(self.fc1(x))
        return x


class EvalModel(nn.Module):
    def __init__(self):
        super(EvalModel, self).__init__()
        self.board = BoardModule()
        self.ft = nn.Linear(256, 128)
        self.l1 = nn.Linear(128, 32)
        self.l2 = nn.Linear(32, 32)
        self.out = nn.Linear(32, 1)

    def forward(self, x: Tensor) -> Tensor:
        x = self.board(x)
        x = F.relu(self.ft(x))
        x = F.relu(self.l1(x))
        x = F.relu(self.l2(x))
        x = self.out(x)
        return x


class LitModule(L.LightningModule):
    def __init__(self):
        super(LitModule, self).__init__()
        self.model = EvalModel()

    def forward(self, x: Tensor) -> Tensor:
        return self.model(x)

    def training_step(self, batch: Tensor, batch_idx: int):
        x, y = batch
        y_hat = self.model(x)
        loss = F.mse_loss(y_hat, y)
        self.log("train_loss", loss)
        return loss

    def validation_step(self, batch: Tensor, batch_idx: int):
        x, y = batch
        y_hat = self.model(x)
        loss = F.mse_loss(y_hat, y)
        self.log("val_loss", loss)
        return loss

    def configure_optimizers(self):
        return Adam(self.parameters(), lr=1e-3)


def main():
    model = LitModule()
    model = model.load_from_checkpoint("model.ckpt")
    model.eval()

    data = torch.load("data.pt")
    dataset = L.TensorDataset(data["x"], data["y"])
    loader = DataLoader(dataset, batch_size=64)

    model.eval()
    with torch.no_grad():
        for x, y in tqdm(loader):
            y_hat = model(x)
            print(F.mse_loss(y_hat, y))


if __name__ == "__main__":
    main()
