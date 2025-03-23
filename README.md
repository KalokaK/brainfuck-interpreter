# Brainfuck Interpreter

This is a simple Brainfuck interpreter written in Rust. Brainfuck is a minimalist programming language with a small set of commands, designed to challenge and amuse programmers.

## Features

- Executes Brainfuck programs.
- Supports custom input files.

## Example Usage

To run a hello-world Brainfuck program, use the following command:

```sh
cargo run -- --source-file <(echo "++++++++++[>+++++++>++++++++++>+++>+<<<<-]>++.>+.+++++++..+++.>++.<<+++++++++++++++.>.+++.------.--------.>+.>.")
```

complete usage can be found with `--help`.

Please also see [brainfuck.org](https://brainfuck.org/) for many more examples.
Heres another example from there, testing both input and output:

```sh
cat src/main.rs | cargo run -- --source-file <(curl https://brainfuck.org/head.b)
```

## TODO
- [ ] When parsing the source code, cache matching brackets.
- [ ] Bevy visualisaiton of how the machine is moving with controls and IO renderer