import torch

def device() -> torch.device:
    return (
        torch.device("cuda")
        if torch.cuda.is_available()
        else torch.device("mps")
        if torch.backends.mps.is_available()
        else torch.device("cpu")
    )

if __name__ == "__main__":
    print(f"Using {device()} device")
