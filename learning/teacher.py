import sqlite3

import torch
import torch.nn as nn
import json
import subprocess
from torch.ao.nn.quantized.modules.linear import Linear
from torch.utils.data import DataLoader
import argparse
from pathlib import Path
import shutil
import io

class Stork:
    def __init__(self, executable):
        self.process = subprocess.Popen(
            [executable],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

        assert self.process.stdin is not None
        assert self.process.stdout is not None

    def get_weights(self, fen):
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        
        self.process.stdin.write(f"position fen {fen}\n")
        self.process.stdin.write("go weights\n")
        self.process.stdin.flush()

        line = self.process.stdout.readline()

        return json.loads(line)

    def close(self):
        self.process.terminate()
        self.process.wait()   
        
class EvalDataset(torch.utils.data.IterableDataset):
    def __init__(self, database_file, stork: Stork) -> None:
        super().__init__()
        database_file = Path(database_file)
        temp_file = database_file.with_name(
            "temp" + database_file.name
        )
        db = sqlite3.connect(database_file)
        self.connection = sqlite3.connect(temp_file)
        db.backup(self.connection)
        db.close()
        self.stork = stork
        
    def __iter__(self):
        cursor = self.connection.execute(
            "SELECT fen, eval_cp FROM positions"
        )
    
        for fen, eval_cp in cursor:
            weights = self.stork.get_weights(fen)
    
            yield (
                torch.tensor(weights, dtype=torch.float32),
                torch.tensor(float(eval_cp), dtype=torch.float32)
            )
    
    def __del__(self):
        self.connection.close()
        
    
class Eval(nn.Module):

    def __init__(self, initial, input_size = 408):
        
        super().__init__()

        self.hidden = nn.Sequential(
            nn.Linear(input_size, 64),
            nn.Softsign(),

            nn.Linear(64,32),
            nn.Softsign(),

            nn.Linear(32, 1)
        )
        hidden1 = self.hidden[0]
        hidden2 = self.hidden[2]
        output = self.hidden[4]
        
        assert isinstance(hidden1, nn.Linear)
        assert isinstance(hidden2, nn.Linear)
        assert isinstance(output, nn.Linear)
        self.main = nn.Linear(input_size, 1)

        self.load_weights(initial)

    def forward(self, x):
        return self.hidden.forward(x) * 100 + self.main.forward(x)

    def load_weights(self, weights):
        hidden1 = self.hidden[0]
        hidden2 = self.hidden[2]
        output = self.hidden[4]
    
        assert isinstance(hidden1, nn.Linear)
        assert isinstance(hidden2, nn.Linear)
        assert isinstance(output, nn.Linear)
    
        with open(weights) as file:
            data = json.load(file)
    
        main_data = data.get("main")
        hidden1_data = data.get("hidden1")
        hidden2_data = data.get("hidden2")
        output_data = data.get("output")
    
        with torch.no_grad():
            if main_data is not None:
                self.main.weight.copy_(
                    torch.tensor(
                        [main_data["weights"]],
                        dtype=self.main.weight.dtype,
                        device=self.main.weight.device,
                    )
                )
    
                assert self.main.bias is not None
                self.main.bias.copy_(
                    torch.tensor(
                        [main_data["bias"]],
                        dtype=self.main.bias.dtype,
                        device=self.main.bias.device,
                    )
                )
    
            if hidden1_data is not None:
                hidden1.weight.copy_(
                    torch.tensor(
                        [neuron["weights"] for neuron in hidden1_data],
                        dtype=hidden1.weight.dtype,
                        device=hidden1.weight.device,
                    )
                )
    
                assert hidden1.bias is not None
                hidden1.bias.copy_(
                    torch.tensor(
                        [neuron["bias"] for neuron in hidden1_data],
                        dtype=hidden1.bias.dtype,
                        device=hidden1.bias.device,
                    )
                )
    
            if hidden2_data is not None:
                hidden2.weight.copy_(
                    torch.tensor(
                        [neuron["weights"] for neuron in hidden2_data],
                        dtype=hidden2.weight.dtype,
                        device=hidden2.weight.device,
                    )
                )
    
                assert hidden2.bias is not None
                hidden2.bias.copy_(
                    torch.tensor(
                        [neuron["bias"] for neuron in hidden2_data],
                        dtype=hidden2.bias.dtype,
                        device=hidden2.bias.device,
                    )
                )
    
            if output_data is not None:
                output.weight.copy_(
                    torch.tensor(
                        [output_data["weights"]],
                        dtype=output.weight.dtype,
                        device=output.weight.device,
                    )
                )
    
                assert output.bias is not None
                output.bias.copy_(
                    torch.tensor(
                        [output_data["bias"]],
                        dtype=output.bias.dtype,
                        device=output.bias.device,
                    )
                )
    
            else:
                nn.init.zeros_(output.weight)
    
                if output.bias is not None:
                    nn.init.zeros_(output.bias)

                    
    def save_weights(self, path):
        hidden1 = self.hidden[0]
        hidden2 = self.hidden[2]
        output = self.hidden[4]
    
        assert isinstance(hidden1, nn.Linear)
        assert isinstance(hidden2, nn.Linear)
        assert isinstance(output, nn.Linear)
    
        def neuron(layer: nn.Linear, i: int):
            assert layer.bias is not None
    
            return {
                "weights": layer.weight[i].detach().cpu().tolist(),
                "bias": layer.bias[i].detach().cpu().item(),
            }
    
        data = {
            "main": neuron(self.main, 0),
    
            "hidden1": [
                neuron(hidden1, i)
                for i in range(hidden1.out_features)
            ],
    
            "hidden2": [
                neuron(hidden2, i)
                for i in range(hidden2.out_features)
            ],
    
            "output": neuron(output, 0),
        }
    
        with open(path, "w") as file:
            json.dump(data, file)
                


def train(data, stork, default_w, epochs, output):
    print("Starting training...")
    model = Eval(default_w, 408)
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    model = model.to(device)
    stork = Stork(stork)
    dataset = EvalDataset(data, stork)
    print(device)
    for parameter in model.main.parameters():
        parameter.requires_grad = False
    
    loader  = torch.utils.data.DataLoader(
        dataset,
        batch_size = 1024,
        num_workers  = 0
    )

    loss_fn = nn.SmoothL1Loss()
    
    optimizer = torch.optim.Adam(
        filter(lambda p: p.requires_grad, model.parameters()),
        lr=1e-3,
    )
    
    model.train()

    for e in range(0, epochs):
        for batch, (x, target) in enumerate(loader):
            x = x.to(device)
            target = target.to(device)
        
            optimizer.zero_grad(set_to_none=True)
        
            prediction = model(x).squeeze(1)
        
            loss = loss_fn(prediction, target)
        
            loss.backward()
        
            optimizer.step()
    
            if batch % 100 == 0:
                print(
                    f"batch={batch}",
                    f"loss={loss.item():.3f}",
                    f"epoch={e}"
                )
            if batch % 500 == 0:
                model.save_weights(unique_path(output))
                print("Saved current weights to a new file")
        model.save_weights(unique_path(output))
        print("Epoch finished. Saved current weights to a new file")

def executable(value: str) -> Path:
     resolved = shutil.which(value)

     if resolved is None:
         raise argparse.ArgumentTypeError(
             f"{value!r} is not an executable"
         )

     return Path(resolved)
 
def unique_path(path: str | Path) -> Path:
    path = Path(path)

    if not path.exists():
        return path

    i = 1
    while True:
        candidate = path.with_name(
            f"{path.stem}{i}{path.suffix}"
        )

        if not candidate.exists():
            return candidate

        i += 1
        
def main(): 
    def main() -> int:
        parser = argparse.ArgumentParser(
            description="Train a model"
        )
    
        parser.add_argument(
            "--data",
            type=Path,
            default="positions.db",
            help="sqlite database file with positions"
        )
    
        parser.add_argument(
            "--stork",
            type=executable,
            required=True,
            help="stork engine executable or fen to weights converter"
        )

        parser.add_argument(
            "--starting-weights",
            type=Path,
            default=Path("default_weights.json")
        )
        parser.add_argument(
              "--epochs",
              default=1,
              type=int,
              help="epoch count"
          )

        parser.add_argument(
                "--output",
                default=unique_path("output"),
                type=Path,
            )
    
        args = parser.parse_args()
    
        if not args.data.is_file():
            parser.error(
                str(args.data)
                + " is not a games database"
            )
     
        try:
            train(
                args.data,
                args.stork,
                args.starting_weights,
                args.epochs,
                args.output
            )
        except KeyboardInterrupt:
            print("\nInterrupted.")
            return 130
        except Exception as err:
            print(
                "An error has occurred: "
                + str(err)
            )
            return 1
    
        return 0
    
    
    if __name__ == "__main__":
        raise SystemExit(main())
    
main()