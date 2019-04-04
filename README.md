# hack-assembler

This is a simple assembler I wrote in Rust as a part of a [nand2tetris course](https://www.nand2tetris.org/course).

Usage:
```
make release
hack-assembler *path-to-asm-file*
```

Planned improvements:
* move main logic to the `lib.rs` module;
* add an option to customize an output file path;
