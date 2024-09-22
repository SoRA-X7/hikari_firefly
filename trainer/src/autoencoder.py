from pathlib import Path
from torch import Tensor, nn
import torch
from torch.optim.adam import Adam
from torch.utils.data import DataLoader, Dataset, random_split
import lightning as L
from tqdm import tqdm
import matplotlib.pyplot as plt
from torchinfo import summary

from data_load import Replay, from_msgpack

W = 10
H = 64

L1 = 64
L2 = 16

# encoder = nn.Sequential(nn.Linear(W * H, L1), nn.ReLU(), nn.Linear(L1, L2))
# decoder = nn.Sequential(nn.Linear(L2, L1), nn.ReLU(), nn.Linear(L1, W * H))

# encoder = nn.Sequential(
#     nn.Flatten(1),
#     nn.Linear(W * H, 256),
#     nn.ReLU(),
#     nn.Linear(256, 128),
#     nn.ReLU(),
#     nn.Linear(128, 64),
#     nn.Sigmoid(),
# )
# decoder = nn.Sequential(
#     nn.Linear(64, 128),
#     nn.ReLU(),
#     nn.Linear(128, 256),
#     nn.ReLU(),
#     nn.Linear(256, W * H),
#     nn.Sigmoid(),
#     nn.Unflatten(1, (1, W, H)),
# )

encoder = nn.Sequential(  # 1x10x64
    nn.Conv2d(1, 16, 3),  # 16x8x62
    nn.MaxPool2d(2),  # 16x4x31
    nn.ReLU(),
    nn.Conv2d(16, 32, 3),  # 32x2x29
    nn.MaxPool2d(2),  # 32x1x14
    nn.ReLU(),
    nn.Flatten(1),
    nn.Dropout(0.2),
    nn.Linear(32 * 1 * 14, 64),
    nn.Sigmoid(),
)
decoder = nn.Sequential(
    nn.Linear(64, 32 * 1 * 14),
    nn.ReLU(),
    nn.Unflatten(1, (32, 1, 14)),
    nn.ConvTranspose2d(32, 16, 3, stride=2),  # 16x4x31
    nn.ReLU(),
    nn.ConvTranspose2d(16, 1, 3, stride=2),  # 1x10x64
    nn.Sigmoid(),
)


# define the LightningModule
class LitAutoEncoder(L.LightningModule):
    def __init__(self, encoder, decoder, *, learning_rate=1e-3):
        super().__init__()
        self.encoder = encoder
        self.decoder = decoder
        self.learning_rate = learning_rate

    def training_step(self, batch, batch_idx):
        # training_step defines the train loop.
        # it is independent of forward
        x = batch
        x = x.unsqueeze(1)
        z = self.encoder(x)
        x_hat = self.decoder(z)
        # print(x, x_hat)
        loss = nn.functional.mse_loss(x_hat, x)
        # Logging to TensorBoard (if installed) by default
        self.log("train_loss", loss, on_epoch=True)
        return loss

    def validation_step(self, batch, batch_idx):
        x = batch
        x = x.unsqueeze(1)
        z = self.encoder(x)
        x_hat = self.decoder(z)
        loss = nn.functional.mse_loss(x_hat, x)
        # Logging to TensorBoard (if installed) by default
        self.log("test_loss", loss)

    def configure_optimizers(self):
        optimizer = Adam(self.parameters(), lr=self.learning_rate)
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

    summary(encoder, (64, 1, 10, 64))
    summary(decoder, (64, 64))
    train_loader = DataLoader(train_set, batch_size=64, shuffle=True)
    valid_loader = DataLoader(valid_set, batch_size=64)
    autoencoder = LitAutoEncoder(encoder, decoder, learning_rate=1e-3)
    # autoencoder = LitAutoEncoder.load_from_checkpoint(
    #     "./lightning_logs/version_49/checkpoints/epoch=29-step=59640.ckpt",
    #     encoder=encoder,
    #     decoder=decoder,
    # )
    trainer = L.Trainer()
    trainer.fit(autoencoder, train_loader, valid_loader)


def load_test():
    with torch.no_grad():
        autoencoder = LitAutoEncoder.load_from_checkpoint(
            "./lightning_logs/version_86/checkpoints/epoch=999-step=217000.ckpt",
            encoder=encoder,
            decoder=decoder,
        )
        dataset = StackerDataset(Path("../data"))
        for data0 in dataset:
            data0 = data0.to(autoencoder.device).unsqueeze(0)
            print(data0)
            hat: Tensor = autoencoder.decoder(autoencoder.encoder(data0.unsqueeze(0)))
            print(hat)
            print(nn.functional.mse_loss(data0, hat))
            fig, axes = plt.subplots(1, 2)
            axes[0].imshow(data0.view(10, 64).T.flip(0).cpu(), cmap="binary_r")
            axes[0].set_title("Original")
            axes[1].imshow(hat.view(10, 64).T.flip(0).cpu(), cmap="binary_r")
            axes[1].set_title("Reconstructed")
            fig.tight_layout()
            plt.show()


if __name__ == "__main__":
    main()
    # load_test()
