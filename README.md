# Chess Engine

A modular chess engine built from scratch in Rust. It supports the UCI protocol, configurable board sizes and an experimental machine-learning evaluator.

The engine currently targets regular chess. The architecture is built to support variants with different board sizes, but very large boards are not fully supported yet.

## Requirements

This project uses Rust nightly because it relies on new Rust features, including `generic_const_exprs`.

Install Rust with [rustup](https://rustup.rs/), then install nightly:

```bash
rustup toolchain install nightly
```

The included `rust-toolchain.toml` selects nightly automatically when you enter the project directory.

## Build and run

Clone the repository and build a release binary:

```bash
git clone https://github.com/gabpa-gaming/chess-engine.git
cd chess-engine
cargo build --release
```

Run the engine directly:

```bash
cargo run --release
```

It communicates through standard input and output using the [UCI protocol](https://www.wbec-ridderkerk.nl/html/UCIProtocol.html). 

Run the tests with:

```bash
cargo test
```

## Using a chess GUI

The easiest way to play against the engine is through a UCI-compatible frontend.

I recommend [Cute Chess](https://cutechess.com/), since it was tested to work well with the engine.

Build the engine first, then add this executable as a UCI engine in the GUI:

```text
target/release/chess-engine
```

On Windows, use `.exe` file instead.

## Using the ML evaluator

The engine starts with the simple material evaluator. It can also load a JSON model through its UCI `Evaluator` option.

The included weight files use the layered evaluator. In your GUI's UCI-engine options, set **Evaluator** to:

```text
layeredml(weights/layered.json)
```

Start the GUI from the project directory, or use an absolute path to the weight file, so the engine can find it. For example:

```text
layeredml(/full/path/to/chess-engine/weights/layered.json)
```

For a single linear JSON model, use this format instead:

```text
ml(path/to/model.json)
```

The Python scripts in [`learning/`](learning/) contain experiments with supervised training and self-play. The training-data generator uses `python-chess`, a UCI teacher engine such as Stockfish, PGN data, SQLite and Zstandard-compressed input.
