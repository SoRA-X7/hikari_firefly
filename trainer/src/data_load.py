from enum import Enum
from typing import Any, Optional
from pydantic import BaseModel
from msgpack import unpack
import sys

import torch


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


class SpinKind(str, Enum):
    none = "none"
    mini = "mini"
    full = "full"


class ReplayState(BaseModel):
    board: list[list[bool]]
    queue: list[str]
    current: str
    unhold: str
    hold: Optional[str]
    ren: int
    b2b: bool
    bag: list[str]


class PieceIdentity(BaseModel):
    cells: list[tuple[int, int]]
    spin: SpinKind


class Replay(BaseModel):
    player_id: int
    frame: int
    state: ReplayState
    action: PieceIdentity

    def into_tensor(self) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        dtype = torch.float32

        board = torch.tensor(self.state.board, dtype=torch.bool).to(dtype)
        meta = torch.cat(
            [
                # current, unhold
                torch.tensor(
                    [
                        _piece_kind_to_ints(self.state.current),
                        _piece_kind_to_ints(self.state.unhold),
                    ],
                    dtype=dtype,
                ).flatten(),
                # queue
                torch.nn.functional.pad(
                    torch.tensor(
                        [_piece_kind_to_ints(kind) for kind in self.state.queue],
                        dtype=dtype,
                    ),
                    (0, 0, 0, 18 - len(self.state.queue)),
                ).flatten(),
                # hold
                torch.tensor(
                    (
                        _piece_kind_to_ints(self.state.hold)
                        if self.state.hold
                        else [0] * 7
                    ),
                    dtype=dtype,
                ).flatten(),
                # bag
                torch.tensor(
                    [
                        1 if "I" in self.state.bag else 0,
                        1 if "O" in self.state.bag else 0,
                        1 if "T" in self.state.bag else 0,
                        1 if "S" in self.state.bag else 0,
                        1 if "Z" in self.state.bag else 0,
                        1 if "J" in self.state.bag else 0,
                        1 if "L" in self.state.bag else 0,
                    ]
                ),
                # ren
                torch.tensor([self.state.ren], dtype=dtype),
                # b2b
                torch.tensor([1 if self.state.b2b else 0], dtype=dtype),
            ]
        )
        action = torch.zeros(10, 64, dtype=dtype)
        for x, y in self.action.cells:
            action[x, y] = 1
        action = torch.cat(
            [
                action.flatten(),
                torch.tensor(
                    [
                        1 if self.action.spin == SpinKind.mini else 0,
                        1 if self.action.spin == SpinKind.full else 0,
                    ],
                    dtype=dtype,
                ),
            ]
        )

        return board, meta, action

    def into_tensor_only_board(self) -> torch.Tensor:
        return torch.tensor(self.state.board, dtype=torch.float32)


def from_msgpack(file_path: str) -> list[Replay]:

    # Open the msgpack file
    with open(file_path, "rb") as file:
        # Decode the msgpack data
        raw: Any = unpack(file)

    data = [Replay.model_validate(replay) for replay in raw]
    # Return the decoded data
    return data


if __name__ == "__main__":
    print(from_msgpack(sys.argv[1])[0].into_tensor())
