import argparse
import io
import random
import shutil
import sqlite3
import subprocess
from enum import Enum
from pathlib import Path

import chess
import chess.engine
import chess.pgn
import zstandard
import hashlib

from queue import Queue
from threading import Thread

SF_WORKERS = 6
SF_THREADS_PER_WORKER = 2
QUEUE_SIZE = 32

TAKE_N_MOVES_OPENING = 1
TAKE_N_MOVES_MIDDLEGAME = 4
TAKE_N_MOVES_ENDGAME = 3

NODES_PER_POSITION = 100_000

DB_PATH = "positions.db"

db = sqlite3.connect(DB_PATH)
db.execute("PRAGMA journal_mode=WAL")
db.execute("PRAGMA synchronous=NORMAL")
db.execute("PRAGMA busy_timeout=60000")
db.close()

def load_existing_hashes() -> set[bytes]:
    conn = sqlite3.connect(DB_PATH)

    try:
        conn.execute("""
            CREATE TABLE IF NOT EXISTS positions (
                hash BLOB PRIMARY KEY,
                fen TEXT NOT NULL,
                eval_cp INTEGER
            ) WITHOUT ROWID;
        """)

        conn.commit()

        print("Loading existing hashes...")

        hashes = {
            row[0]
            for row in conn.execute(
                "SELECT hash FROM positions"
            )
        }

        print(f"Loaded {len(hashes):,} existing positions.")

        return hashes

    finally:
        conn.close()


def generate(teacher, games, start_percent: float):
    task_queue = Queue(maxsize=QUEUE_SIZE)
    result_queue = Queue(maxsize=QUEUE_SIZE * 4)

    known_hashes = load_existing_hashes()

    file_size = games.stat().st_size
    start_byte = int(file_size * start_percent / 100.0)

    if start_percent > 0:
        print(
            f"Skipping to approximately {start_percent:.2f}% "
            f"({start_byte / 1024**3:.2f} GiB compressed)..."
        )

    def stockfish_worker(worker_id: int):
        engine = chess.engine.SimpleEngine.popen_uci(str(teacher))

        engine.configure({
            "Threads": SF_THREADS_PER_WORKER
        })

        try:
            while True:
                item = task_queue.get()

                try:
                    if item is None:
                        return

                    h, pos = item

                    board = chess.Board(pos)

                    info = engine.analyse(
                        board,
                        chess.engine.Limit(
                            nodes=NODES_PER_POSITION
                        )
                    )

                    score_info = info.get("score")

                    if score_info is None:
                        continue

                    score = score_info.white().score(
                        mate_score=1000
                    )

                    if score is None:
                        continue

                    result_queue.put(
                        (h, pos, score)
                    )

                finally:
                    task_queue.task_done()

        finally:
            engine.quit()

    def db_writer():
        writer_db = sqlite3.connect(
            DB_PATH,
            timeout=60
        )

        writer_db.execute(
            "PRAGMA busy_timeout = 60000"
        )

        added = 0

        try:
            while True:
                item = result_queue.get()

                try:
                    if item is None:
                        return

                    h, pos, score = item

                    cursor = writer_db.execute(
                        """
                        INSERT OR IGNORE INTO positions
                            (hash, fen, eval_cp)
                        VALUES (?, ?, ?)
                        """,
                        (h, pos, score)
                    )

                    if cursor.rowcount == 1:
                        added += 1

                        print(
                            ".",
                            end="",
                            flush=True
                        )

                        if added % 300 == 0:
                            writer_db.commit()

                finally:
                    result_queue.task_done()

        finally:
            writer_db.commit()
            writer_db.close()

            print(
                f"\nWriter finished. "
                f"Added {added:,} positions."
            )

    workers = [
        Thread(
            target=stockfish_worker,
            args=(i,)
        )
        for i in range(SF_WORKERS)
    ]

    writer = Thread(target=db_writer)

    writer.start()

    for worker in workers:
        worker.start()

    reached_start = start_percent == 0

    with open(games, "rb") as compressed:
        decompressor = zstandard.ZstdDecompressor()

        with decompressor.stream_reader(
            compressed
        ) as binary_stream:

            with io.TextIOWrapper(
                binary_stream,
                encoding="utf-8"
            ) as text_stream:

                while True:
                    game = chess.pgn.read_game(
                        text_stream
                    )

                    if game is None:
                        break

                    if not reached_start:
                        current_byte = compressed.tell()

                        if current_byte < start_byte:
                            continue

                        reached_start = True

                        actual_percent = (
                            current_byte
                            / file_size
                            * 100
                        )

                        print(
                            f"Reached ~{actual_percent:.2f}%. "
                            f"Starting generation."
                        )

                    board = game.board()

                    positions = {
                        GamePhase.OPENING: [],
                        GamePhase.MIDDLEGAME: [],
                        GamePhase.ENDGAME: [],
                    }

                    for move in game.mainline_moves():
                        board.push(move)

                        phase = get_game_phase(board)

                        positions[phase].append(
                            board.fen()
                        )

                    chosen_positions = (
                        random.sample(
                            positions[GamePhase.OPENING],
                            min(
                                TAKE_N_MOVES_OPENING,
                                len(
                                    positions[
                                        GamePhase.OPENING
                                    ]
                                )
                            )
                        )
                        +
                        random.sample(
                            positions[
                                GamePhase.MIDDLEGAME
                            ],
                            min(
                                TAKE_N_MOVES_MIDDLEGAME,
                                len(
                                    positions[
                                        GamePhase.MIDDLEGAME
                                    ]
                                )
                            )
                        )
                        +
                        random.sample(
                            positions[GamePhase.ENDGAME],
                            min(
                                TAKE_N_MOVES_ENDGAME,
                                len(
                                    positions[
                                        GamePhase.ENDGAME
                                    ]
                                )
                            )
                        )
                    )

                    for pos in chosen_positions:
                        h = fen_hash(pos)

                        if h in known_hashes:
                            continue

                        known_hashes.add(h)

                        task_queue.put(
                            (h, pos)
                        )


    for _ in workers:
        task_queue.put(None)

    task_queue.join()

    for worker in workers:
        worker.join()

    result_queue.put(None)

    result_queue.join()

    writer.join()

def fen_hash(fen: str) -> bytes:
    return hashlib.blake2b(
        fen.encode(),
        digest_size=16
    ).digest()

def get_game_phase(board: chess.Board):
    n = chess.popcount(board.knights)
    b = chess.popcount(board.bishops)
    q = chess.popcount(board.queens)
    r = chess.popcount(board.rooks)
    sum = n+b+q+r
    if sum <= 12 or q == 0 or board.ply() > 12:
        heavy = q + r
        if heavy <= 2:
            return GamePhase.ENDGAME
        return GamePhase.MIDDLEGAME
    return GamePhase.OPENING



class GamePhase(Enum):
    OPENING = 1
    MIDDLEGAME = 2
    ENDGAME = 3

def executable(value: str) -> Path:
     resolved = shutil.which(value)

     if resolved is None:
         raise argparse.ArgumentTypeError(
             f"{value!r} is not an executable"
         )

     return Path(resolved)

def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate training data for supervised learning."
    )

    parser.add_argument(
        "--teacher",
        type=executable,
        required=True,
        help="path to the UCI engine executable"
    )

    parser.add_argument(
        "--games",
        type=Path,
        required=True,
        help=".pgn.zst games file"
    )

    parser.add_argument(
        "--start-percent",
        type=float,
        default=0.0,
        help="ignore games until approximately this percentage "
             "of the compressed PGN file (0-100)"
    )

    args = parser.parse_args()

    if not args.teacher.is_file():
        parser.error(
            str(args.teacher)
            + " is not an engine executable"
        )

    if not args.games.is_file():
        parser.error(
            str(args.games)
            + " is not a games file"
        )

    if not 0.0 <= args.start_percent <= 100.0:
        parser.error(
            "--start-percent must be between 0 and 100"
        )

    try:
        generate(
            args.teacher,
            args.games,
            args.start_percent
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
