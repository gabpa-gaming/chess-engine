#!/usr/bin/env python3

import argparse
import copy
import json
import math
import queue
import random
import subprocess
import sys
import threading
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, TextIO

INPUT_SIZE = 408
HIDDEN1_SIZE = 64
HIDDEN2_SIZE = 32

MIN_MUT_C = 10
MAX_MUT_C = 50

SMALL_MUT_CHANCE = 0.6
MED_MUT_CHANCE = 0.20
BIG_MUT_CHANCE = 0.05

SMALL_MUT_RANGE = 0.1
MED_MUT_RANGE = 0.5
BIG_MUT_RANGE = 1

DEFAULT_OUTPUT_DIR = Path(__file__).with_name("generations")
PIECE_VALUES = {
    1: 1,  # pawn
    2: 3,  # knight
    3: 3,  # bishop
    4: 5,  # rook
    5: 9,  # queen
}


def validate_linear_model(model: Any, input_size: int, name: str) -> dict[str, Any]:
    if not isinstance(model, dict) or set(model) != {"weights", "bias"}:
        raise ValueError(f"{name} must contain only 'weights' and 'bias'")
    values = model["weights"]
    if not isinstance(values, list) or len(values) != input_size:
        raise ValueError(f"{name}.weights must be an array of {input_size} numbers")
    if not all(isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) for value in values):
        raise ValueError(f"{name}.weights must contain only finite numbers")
    bias = model["bias"]
    if not isinstance(bias, (int, float)) or isinstance(bias, bool) or not math.isfinite(bias):
        raise ValueError(f"{name}.bias must be a finite number")
    return {"weights": [float(value) for value in values], "bias": float(bias)}


def load_weights(path: Path) -> dict[str, Any]:
    try:
        with path.open(encoding="utf-8") as file:
            model = json.load(file)
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read weights file {path}: {error}") from error

    if not isinstance(model, dict):
        raise ValueError("weights JSON must be an object")
    if set(model) == {"weights", "bias"}:
        return validate_linear_model(model, INPUT_SIZE, "model")
    if set(model) != {"main", "hidden1", "hidden2", "output"}:
        raise ValueError("model must be a linear model or a layered network")

    main = validate_linear_model(model["main"], INPUT_SIZE, "main")
    hidden1 = model["hidden1"]
    hidden2 = model["hidden2"]
    if not isinstance(hidden1, list) or len(hidden1) != HIDDEN1_SIZE:
        raise ValueError(f"hidden1 must contain {HIDDEN1_SIZE} neurons")
    if not isinstance(hidden2, list) or len(hidden2) != HIDDEN2_SIZE:
        raise ValueError(f"hidden2 must contain {HIDDEN2_SIZE} neurons")
    return {
        "main": main,
        "hidden1": [validate_linear_model(neuron, INPUT_SIZE, f"hidden1[{index}]") for index, neuron in enumerate(hidden1)],
        "hidden2": [validate_linear_model(neuron, HIDDEN1_SIZE, f"hidden2[{index}]") for index, neuron in enumerate(hidden2)],
        "output": validate_linear_model(model["output"], HIDDEN2_SIZE, "output"),
    }


def save_weights(path: Path, model: dict[str, Any]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(model, indent=2) + "\n", encoding="utf-8")
    return path.resolve()


def heuristic_main_model() -> dict[str, Any]:
    weights: list[float] = []
    for piece in range(6):
        for square in range(64):
            file = square % 8
            rank = 8 - square // 8
            center = 3.5 - abs(file - 3.5) - abs(rank - 3.5)
            if piece == 0:
                value = rank * 3.0 + center
            elif piece == 1:
                value = rank + center * 0.5
            elif piece == 2:
                value = center * 5.0
            elif piece == 3:
                value = center * 3.0
            elif piece == 4:
                value = center * 1.5
            else:
                value = -center * 3.0
                if rank == 1 and file in {2, 6}:
                    value += 12.0
            weights.append(value)
    weights.extend([0.0] * 24)
    return {"weights": weights, "bias": 0.0}


def heuristic_model() -> dict[str, Any]:
    def zero_model(input_size: int) -> dict[str, Any]:
        return {"weights": [0.0] * input_size, "bias": 0.0}

    return {
        "main": heuristic_main_model(),
        "hidden1": [zero_model(INPUT_SIZE) for _ in range(HIDDEN1_SIZE)],
        "hidden2": [zero_model(HIDDEN1_SIZE) for _ in range(HIDDEN2_SIZE)],
        "output": zero_model(HIDDEN2_SIZE),
    }

def choose_mutation_range() -> float:
    r = random.random()

    if r < SMALL_MUT_CHANCE:
        return SMALL_MUT_RANGE
    elif r < SMALL_MUT_CHANCE + MED_MUT_CHANCE:
        return MED_MUT_RANGE
    elif r < SMALL_MUT_CHANCE + MED_MUT_CHANCE + BIG_MUT_CHANCE:
        return BIG_MUT_RANGE

    return 0.0

def numeric_parameters(value: Any) -> list[tuple[list[Any] | dict[str, Any], int | str]]:
    parameters: list[tuple[list[Any] | dict[str, Any], int | str]] = []
    if isinstance(value, dict):
        for key, child in value.items():
            if isinstance(child, (dict, list)):
                parameters.extend(numeric_parameters(child))
            else:
                parameters.append((value, key))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            if isinstance(child, (dict, list)):
                parameters.extend(numeric_parameters(child))
            else:
                parameters.append((value, index))
    return parameters


def mutate(model: dict[str, Any], mut_range: float) -> dict[str, Any]:
    child = copy.deepcopy(model)
    parameters = numeric_parameters(child)
    for container, key in random.sample(parameters, min(len(parameters), random.randint(MIN_MUT_C, MAX_MUT_C))):
        container[key] += random.uniform(-mut_range, mut_range)
    return child


def average_models(players: list[dict[str, Any]]) -> dict[str, Any]:
    models = [load_weights(Path(player["weights"])) for player in players]
    winrates = [((player["wins"] + player["draws"] / 2) / (player["wins"] + player["losses"] + player["draws"])) ** 2 for player in players]
    wr_sum = sum(winrates)
    ratios = [wr / wr_sum for wr in winrates]

    def average(values: list[Any]) -> Any:
        first = values[0]
        if isinstance(first, dict):
            if any(not isinstance(value, dict) or value.keys() != first.keys() for value in values):
                raise ValueError("cannot average models with different shapes")
            return {key: average([value[key] for value in values]) for key in first}
        if isinstance(first, list):
            if any(not isinstance(value, list) or len(value) != len(first) for value in values):
                raise ValueError("cannot average models with different shapes")
            return [average([value[index] for value in values]) for index in range(len(first))]
        if any(not isinstance(value, (int, float)) for value in values):
            raise ValueError("cannot average models with different shapes")
        return sum(value * ratio for value, ratio in zip(values, ratios))

    return average(models)


def new_player(model: dict[str, Any], output_dir: Path) -> dict[str, Any]:
    player_id = str(uuid.uuid4())
    weights = save_weights(output_dir / "players" / f"{player_id}.json", model)
    return {
        "id": player_id,
        "weights": str(weights),
        "evaluator": "layeredml" if "main" in model else "ml",
        "wins": 0,
        "losses": 0,
        "draws": 0,
        "score": 0.0,
    }


class UciPlayer:
    def __init__(self, engine: Path, player: dict[str, Any], depth: int) -> None:
        self.player = player
        self.depth = depth
        self.process = subprocess.Popen(
            [str(engine)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        assert self.process.stdin and self.process.stdout
        self.lines: queue.Queue[str | None] = queue.Queue()
        threading.Thread(target=self._read_lines, args=(self.process.stdout,), daemon=True).start()
        self.send("uci")
        self.wait_for("uciok")
        self.set_player(player)

    def set_player(self, player: dict[str, Any]) -> None:
        weights = Path(player["weights"]).resolve()
        evaluator = player.get("evaluator")
        if evaluator is None:
            evaluator = "layeredml" if "main" in load_weights(weights) else "ml"
        self.send(f"setoption name Evaluator value {evaluator}({weights})")
        self.send(f"setoption name Depth value {self.depth}")
        self.send("isready")
        self.wait_for("readyok")

    def _read_lines(self, stream: TextIO) -> None:
        for line in stream:
            self.lines.put(line.strip())
        self.lines.put(None)

    def send(self, command: str) -> None:
        if self.process.poll() is not None:
            raise RuntimeError(f"engine exited with status {self.process.returncode}")
        assert self.process.stdin
        self.process.stdin.write(command + "\n")
        self.process.stdin.flush()

    def wait_for(self, expected: str) -> None:
        while True:
            try:
                line = self.lines.get(timeout=10)
            except queue.Empty as error:
                raise RuntimeError(f"timed out waiting for {expected}") from error
            if line is None:
                raise RuntimeError(f"engine stopped while waiting for {expected}")
            if line == expected:
                return

    def best_move(self, moves: list[str]) -> str:
        position = "position startpos" + (" moves " + " ".join(moves) if moves else "")
        self.send(position)
        self.send("go")
        while True:
            try:
                line = self.lines.get(timeout=30)
            except queue.Empty as error:
                raise RuntimeError("timed out waiting for bestmove") from error
            if line is None:
                raise RuntimeError("engine stopped while searching")
            if line.startswith("bestmove "):
                return line.split(maxsplit=1)[1]

    def close(self) -> None:
        if self.process.poll() is None:
            self.send("quit")
            self.process.wait(timeout=5)


def describe_move(move: str) -> str:
    if len(move) < 4:
        return move
    description = f"{move[:2]} to {move[2:4]}"
    if len(move) == 5:
        description += f", promote to {move[4].upper()}"
    return description


def random_opening_pawn_move(ply: int) -> str:
    file = random.choice("abcdefgh")
    if ply == 0:
        return f"{file}2{file}{random.choice((3, 4))}"
    return f"{file}7{file}{random.choice((5, 6))}"


def result_at_ply_limit(moves: list[str]) -> str:
    try:
        import chess
    except ImportError as error:
        raise RuntimeError("python-chess is required to score material at the ply limit") from error

    board = chess.Board()
    for move in moves:
        board.push_uci(move)

    material = {
        color: sum(
            len(board.pieces(piece, color)) * value
            for piece, value in PIECE_VALUES.items()
        )
        for color in (chess.WHITE, chess.BLACK)
    }
    difference = material[chess.WHITE] - material[chess.BLACK]
    if difference >= 3:
        return "1-0"
    if difference <= -3:
        return "0-1"
    return "1/2-1/2"


def append_log(log_file: Path, record: dict[str, Any]) -> None:
    record["timestamp"] = datetime.now(timezone.utc).isoformat()
    with log_file.open("a", encoding="utf-8") as file:
        file.write(json.dumps(record) + "\n")


def play_game(
    white: dict[str, Any],
    black: dict[str, Any],
    engine: Path,
    depth: int,
    max_plies: int,
    log_file: Path,
    generation: int,
) -> tuple[str, list[str], str]:
    white_engine, black_engine = UciPlayer(engine, white, depth), UciPlayer(engine, black, depth)
    try:
        import chess
    except ImportError as error:
        white_engine.close()
        black_engine.close()
        raise RuntimeError("python-chess is required to validate match moves") from error

    board = chess.Board()
    moves: list[str] = []
    game_id = str(uuid.uuid4())
    try:
        white_engine.send("ucinewgame")
        black_engine.send("ucinewgame")
        for ply in range(max_plies):
            color, player, player_engine = (
                ("White", white, white_engine) if ply % 2 == 0 else ("Black", black, black_engine)
            )
            selection = "random_pawn" if ply < 2 else "engine"
            move = random_opening_pawn_move(ply) if selection == "random_pawn" else player_engine.best_move(moves)
            if move in {"0000", "(none)"}:
                return ("0-1" if ply % 2 == 0 else "1-0"), moves, game_id
            try:
                board.push_uci(move)
            except ValueError as error:
                raise RuntimeError(f"engine returned an illegal move {move!r}") from error
            moves.append(move)
            description = describe_move(move)
            append_log(
                log_file,
                {
                    "event": "move",
                    "game_id": game_id,
                    "generation": generation,
                    "ply": ply + 1,
                    "color": color,
                    "player_id": player["id"],
                    "move": move,
                    "description": description,
                    "selection": selection,
                },
            )
            print(f"{color} {selection} move. Player: {player['id']}. {description}")
            if board.is_game_over(claim_draw=False):
                return board.result(claim_draw=False), moves, game_id
        return result_at_ply_limit(moves), moves, game_id
    finally:
        white_engine.close()
        black_engine.close()


def update_scores(white: dict[str, Any], black: dict[str, Any], result: str) -> None:
    if result == "1-0":
        white["wins"] += 1
        white["score"] += 1.0
        black["losses"] += 1
    elif result == "0-1":
        black["wins"] += 1
        black["score"] += 1.0
        white["losses"] += 1
    else:
        white["draws"] += 1
        black["draws"] += 1
        white["score"] += 0.5
        black["score"] += 0.5


def log_game(
    log_file: Path,
    generation: int,
    game_id: str,
    result: str,
    moves: list[str],
    white: dict[str, Any],
    black: dict[str, Any],
) -> None:
    record = {
        "event": "game_complete",
        "game_id": game_id,
        "generation": generation,
        "result": result,
        "moves": moves,
        "white": {"id": white["id"], "weights": white["weights"]},
        "black": {"id": black["id"], "weights": black["weights"]},
    }
    append_log(log_file, record)


def play_game_with_retries(
    white: dict[str, Any],
    black: dict[str, Any],
    engine: Path,
    depth: int,
    max_plies: int,
    log_file: Path,
    generation: int,
    retries: int,
) -> tuple[str, list[str], str]:
    for attempt in range(retries + 1):
        try:
            return play_game(white, black, engine, depth, max_plies, log_file, generation)
        except (OSError, RuntimeError, subprocess.SubprocessError) as error:
            if attempt == retries:
                raise
            print(f"Game failed ({error}); restarting engines, retry {attempt + 1}/{retries}.", file=sys.stderr)
            append_log(
                log_file,
                {
                    "event": "game_restart",
                    "generation": generation,
                    "attempt": attempt + 1,
                    "white_id": white["id"],
                    "black_id": black["id"],
                    "error": str(error),
                },
            )
    raise AssertionError("retry loop must return or raise")


def load_state(state_file: Path) -> dict[str, Any] | None:
    if not state_file.exists():
        return None
    try:
        with state_file.open(encoding="utf-8") as file:
            state = json.load(file)
        if not isinstance(state.get("generation"), int) or len(state.get("players", [])) < 4:
            raise ValueError("missing generation or players")
        return state
    except (OSError, json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"cannot read generation state {state_file}: {error}") from error


def create_population(
    state: dict[str, Any] | None,
    generation_dir: Path,
    gen_size: int,
    initial: dict[str, Any] | None,
) -> list[dict[str, Any]]:

    if state is None:
        base = initial if initial is not None else heuristic_model()

        return [
            new_player(
                mutate(base, choose_mutation_range()),
                generation_dir,
            )
            for _ in range(gen_size)
        ]

    elite = sorted(
        state["players"],
        key=lambda player: (player["score"], player["wins"]),
        reverse=True,
    )[:4]

    seed = average_models(elite)
    save_weights(generation_dir / "average_top_four.json", seed)

    retained = [
        {
            **player,
            "wins": 0,
            "losses": 0,
            "draws": 0,
            "score": 0.0,
        }
        for player in elite
    ]

    return retained + [
        new_player(
            mutate(seed, choose_mutation_range()),
            generation_dir,
        )
        for _ in range(gen_size - len(retained))
    ]

def main() -> int:
    parser = argparse.ArgumentParser(description="Evolve JSON LinearModel weights with UCI round-robin matches.")
    parser.add_argument("engine", type=Path, help="path to the UCI engine executable")
    parser.add_argument("--gen-size", type=int, default=16, help="players per generation, from 12 to 16 (default: 16)")
    parser.add_argument("--depth", type=int, default=5, help="fixed UCI search depth for every engine move (default: 5)")
    parser.add_argument("--max-plies", type=int, default=200, help="draw games after this many plies")
    parser.add_argument("--retries", type=int, default=3, help="game retries after an engine failure (default: 3)")
    parser.add_argument("--initial-weights", type=Path, help="optional initial JSON model for the first generation")
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT_DIR, help="directory for generated models and logs")
    parser.add_argument("--start-generation", type=int, help="resume from the completed generation number")
    args = parser.parse_args()

    if not args.engine.is_file():
        parser.error(f"engine is not a file: {args.engine}")
    if not 12 <= args.gen_size <= 16:
        parser.error("--gen-size must be between 12 and 16")
    if not 1 <= args.depth <= 20:
        parser.error("--depth must be between 1 and 20")
    if args.max_plies < 1 or args.retries < 0:
        parser.error("--max-plies must be positive and --retries cannot be negative")
    if args.start_generation is not None and args.start_generation < 0:
        parser.error("--start-generation cannot be negative")

    try:
        initial = load_weights(args.initial_weights) if args.initial_weights else None
        args.output_dir.mkdir(parents=True, exist_ok=True)
        state_file = args.output_dir / "state.json"
        if args.start_generation is None:
            state = load_state(state_file)
        else:
            selected_state = args.output_dir / f"generation_{args.start_generation:04d}" / "state.json"
            state = load_state(selected_state)
            if state is None or state["generation"] != args.start_generation:
                raise ValueError(f"no completed state for generation {args.start_generation}")
        while True:
            generation = 0 if state is None else state["generation"] + 1
            generation_dir = args.output_dir / f"generation_{generation:04d}"
            players = create_population(state, generation_dir, args.gen_size, initial)
            log_file = generation_dir / "games.jsonl"
            for white_index, white in enumerate(players):
                for black in players[white_index + 1:]:
                    for first, second in ((white, black), (black, white)):
                        result, moves, game_id = play_game_with_retries(
                            first,
                            second,
                            args.engine.resolve(),
                            args.depth,
                            args.max_plies,
                            log_file,
                            generation,
                            args.retries,
                        )
                        update_scores(first, second, result)
                        log_game(log_file, generation, game_id, result, moves, first, second)
                        if result == "1-0":
                            print(f"White won. Player: {first['id']}")
                        elif result == "0-1":
                            print(f"Black won. Player: {second['id']}")
                        else:
                            print("Draw.")
            ranking = sorted(players, key=lambda player: (player["score"], player["wins"]), reverse=True)
            average = average_models(ranking[:4])
            average_path = save_weights(generation_dir / "average_top_four.json", average)
            state = {"generation": generation, "players": players, "average_weights": str(average_path)}
            state_json = json.dumps(state, indent=2) + "\n"
            (generation_dir / "state.json").write_text(state_json, encoding="utf-8")
            state_file.write_text(state_json, encoding="utf-8")
            print(f"Generation {generation} complete. Best player: {ranking[0]['id']} ({ranking[0]['score']} points)")
    except KeyboardInterrupt:
        print("Evolution stopped. Resume from the latest completed generation with the same command.")
        return 0
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        print(f"evolution failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
