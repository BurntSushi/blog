all:
	@echo Please pick a target.

deploy:
	hugo
	rscp public/ gopher:/home/andrew/www/burntsushi.net/blog

clean:
	rm -rf public

toml:
	cargo build --manifest-path ./code/rust-error-handling/Cargo.toml
	./scripts/to-toml ./code/rust-error-handling/src/bin/*.rs \
		> ./data/code/rusterrorhandling.toml

push:
	git push origin master
	git push github master
