all:
	@echo Please pick a target.

deploy:
	hugo
	rscp public/ bs:/home/ubuntu/www/burntsushi.net/blog

server:
	hugo server -D -w --bind '192.168.1.212' -b 'http://192.168.1.212/'

clean:
	rm -rf tmp
	rm -f static/images/transducers/{dot,sets,maps}/*.png

code-error:
	./scripts/rust-from-blog content/post/rust-error-handling.md
	cargo build --manifest-path ./code/rust-error-handling/Cargo.toml

IMG_HAND_WRITTEN = $(foreach f,$(wildcard blogdata/transducers/dot/*.dot), static/images/transducers/dot/$(subst .dot,.png,$(notdir $f)))
IMG_SET = $(foreach f,$(wildcard blogdata/transducers/sets/*), static/images/transducers/sets/$(addsuffix .png,$(notdir $f)))
IMG_MAP = $(foreach f,$(wildcard blogdata/transducers/maps/*), static/images/transducers/maps/$(addsuffix .png,$(notdir $f)))

code-transducers:
	./scripts/rust-from-blog content/post/transducers.md
	cargo build --release --manifest-path ./code/transducers/Cargo.toml

img-transducers: $(IMG_HAND_WRITTEN) $(IMG_SET) $(IMG_MAP)

static/images/transducers/dot/%.png: blogdata/transducers/dot/%.dot
	mkdir -p $(dir $@)
	dot -Tpng $< > $@

static/images/transducers/sets/%.png: tmp/blogdata/transducers/sets/%.dot
	mkdir -p $(dir $@)
	dot -Tpng $< > $@

tmp/blogdata/transducers/sets/%.dot: tmp/blogdata/transducers/sets/%.fst
	mkdir -p $(dir $@)
	fst dot --state-names $< > $@

tmp/blogdata/transducers/sets/%.fst: blogdata/transducers/sets/%
	mkdir -p $(dir $@)
	fst set --sorted $< $@

static/images/transducers/maps/%.png: tmp/blogdata/transducers/maps/%.dot
	mkdir -p $(dir $@)
	dot -Tpng $< > $@

tmp/blogdata/transducers/maps/%.dot: tmp/blogdata/transducers/maps/%.fst
	mkdir -p $(dir $@)
	fst dot --state-names $< > $@

tmp/blogdata/transducers/maps/%.fst: blogdata/transducers/maps/%
	mkdir -p $(dir $@)
	fst map --sorted $< $@

push:
	git push origin master
	git push github master

.PHONY: code
