.PHONY: clean run debug

run:
	RUSTFLAGS='-Clink-args=-Tsrc/lds/virt.lds --cfg gdb="false"' cargo run $(args)

debug:
	RUSTFLAGS='-Clink-args=-Tsrc/lds/virt.lds --cfg gdb="true"' cargo run $(args)

clean:
	cargo clean