.PHONY: clean run debug

run:
	RUSTFLAGS='-Clink-args=-Tsrc/lds/virt.lds --cfg gdb="false"' cargo run

debug:
	RUSTFLAGS='-Clink-args=-Tsrc/lds/virt.lds --cfg gdb="true"' cargo run

clean:
	cargo clean