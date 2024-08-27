# dumpfiles

`dumpfiles` is a Rust command-line tool to generate a structured text representation of a directory's contents, including file trees and the contents of text files.
It's designed to make it easy to share an entire code repository or any set of text files to an LLM.

## Features

- Generates a hierarchical representation of directory structures
- Includes file contents for text files
- Supports custom ignore patterns
- Integrates with `.gitignore` files

## Installation

To install `dumpfiles`, you need to have Rust and Cargo installed on your system. Then, you can build the project from source:

```bash
cargo install dumpfiles
```

## Usage

```bash
dumpfiles [OPTIONS] <DIRECTORY>
```

### Arguments

- `<DIRECTORY>`: Path to the directory to process

### Options

- `-o, --output <FILE>`: Path to the output file (default: "output.txt")
- `-i, --ignore <PATTERN>`: Patterns to ignore (can be used multiple times, default: ".git*")
- `-g, --gitignore <FILE>`: Path to the .gitignore file (default: ".gitignore")
- `--no-gitignore`: Ignore the .gitignore file
- `-h, --help`: Print help information
- `-V, --version`: Print version information

### Example

```bash
dumpfiles /path/to/your/project -o project_dump.txt -i "*.log" -i "node_modules*"
```

This command will process the `/path/to/your/project` directory, ignore all `.log` files and the `node_modules` directory, and save the output to `project_dump.txt`.

## Output Format

The output file contains two main sections:

1. A tree representation of the directory structure
2. The contents of each file, nested within XML-like tags representing the directory structure

Example:

```
<tree>
project/
    src/
        main.rs
    Cargo.toml
</tree>

<src>
<main.rs>
fn main() {
    println!("Hello, world!");
}
</main.rs>
</src>
<Cargo.toml>
[package]
name = "project"
version = "0.1.0"
edition = "2021"

[dependencies]
</Cargo.toml>
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
