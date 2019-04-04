release:
	cargo build --release
	ln -s $(pwd)/target/release/assembler ~/.cargo/bin/hack-assembler
