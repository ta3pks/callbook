.PHONY: deploy 
target/x86_64-unknown-linux-gnu/release/callbook: $(shell find src -type f -name '*.rs')
	TARGET_CC=x86_64-unknown-linux-gnu-gcc cargo build --release --target=x86_64-unknown-linux-gnu
deploy: target/x86_64-unknown-linux-gnu/release/callbook
	rsync -vaurz $< $(SERVER):/usr/local/bin/callbook
