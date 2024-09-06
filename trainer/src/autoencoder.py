from pathlib import Path
from torch import Tensor, nn
import torch
from torch.optim.adam import Adam
from torch.utils.data import DataLoader, Dataset, random_split
import lightning as L
from tqdm import tqdm
import matplotlib.pyplot as plt

from data_load import Replay, from_msgpack

W = 10
H = 64

L1 = 64
L2 = 16

encoder = nn.Sequential(nn.Linear(W * H, L1), nn.ReLU(), nn.Linear(L1, L2))
decoder = nn.Sequential(nn.Linear(L2, L1), nn.ReLU(), nn.Linear(L1, W * H))


# define the LightningModule
class LitAutoEncoder(L.LightningModule):
    def __init__(self, encoder, decoder):
        super().__init__()
        self.encoder = encoder
        self.decoder = decoder

    def training_step(self, batch, batch_idx):
        # training_step defines the train loop.
        # it is independent of forward
        x = batch
        x = x.view(x.size(0), -1)
        z = self.encoder(x)
        x_hat = self.decoder(z)
        # print(x, x_hat)
        loss = nn.functional.mse_loss(x_hat, x)
        # Logging to TensorBoard (if installed) by default
        self.log("train_loss", loss, on_epoch=True)
        return loss

    def validation_step(self, batch, batch_idx):
        x = batch
        x = x.view(x.size(0), -1)
        z = self.encoder(x)
        x_hat = self.decoder(z)
        loss = nn.functional.mse_loss(x_hat, x)
        # Logging to TensorBoard (if installed) by default
        self.log("test_loss", loss)

    def configure_optimizers(self):
        optimizer = Adam(self.parameters(), lr=3e-3)
        return optimizer


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

    def __getitem__(self, idx) -> Tensor:
        return self.data[idx].into_tensor_only_board()


def main():
    dataset = StackerDataset(Path("../data"))

    # use 20% of training data for validation
    train_set_size = int(len(dataset) * 0.8)
    valid_set_size = len(dataset) - train_set_size
    # split the train set into two
    seed = torch.Generator().manual_seed(42)
    train_set, valid_set = random_split(
        dataset, [train_set_size, valid_set_size], generator=seed
    )
    print(len(train_set), len(valid_set))

    train_loader = DataLoader(train_set)
    valid_loader = DataLoader(valid_set)
    autoencoder = LitAutoEncoder(encoder, decoder)
    # autoencoder = LitAutoEncoder.load_from_checkpoint(
    #     "./lightning_logs/version_23/checkpoints/epoch=19-step=39760.ckpt",
    #     encoder=encoder,
    #     decoder=decoder,
    # )
    trainer = L.Trainer(max_epochs=30)
    trainer.fit(autoencoder, train_loader, valid_loader)


def load_test():
    with torch.no_grad():
        autoencoder = LitAutoEncoder.load_from_checkpoint(
            "./lightning_logs/version_26/checkpoints/epoch=29-step=59640.ckpt",
            encoder=encoder,
            decoder=decoder,
        )
        dataset = StackerDataset(Path("../data"))
        for data0 in dataset:
            data0 = data0.to(autoencoder.device).view(1, -1)
            hat: Tensor = autoencoder.decoder(autoencoder.encoder(data0))
            print(data0)
            print(hat.view(10, 64))
            print(nn.functional.mse_loss(data0, hat))
            fig, axes = plt.subplots(1, 2)
            axes[0].imshow(data0.view(10, 64).T.flip(0).cpu(), cmap="binary_r")
            axes[1].imshow(hat.view(10, 64).T.flip(0).cpu(), cmap="binary_r")
            fig.tight_layout()
            plt.show()


if __name__ == "__main__":
    # main()
    load_test()
