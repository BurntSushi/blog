all:
	@echo Please pick a target.

deploy:
	hugo
	rscp public/ gopher:/home/andrew/www/burntsushi.net/blog

clean:
	rm -rf public

code:
	./scripts/rust-from-blog content/post/rust-error-handling.md
	cargo build --manifest-path ./code/rust-error-handling/Cargo.toml

push:
	git push origin master
	git push github master

.PHONY: code
