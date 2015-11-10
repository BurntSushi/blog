all:
	@echo Please pick a target.

deploy:
	hugo
	rscp public/ gopher:/home/andrew/www/burntsushi.net/blog

clean:
	rm -rf tmp
	rm -f static/images/transducers/{dot,sets,maps}/*.png

code-error:
	./scripts/rust-from-blog content/post/rust-error-handling.md
	cargo build --manifest-path ./code/rust-error-handling/Cargo.toml

IMG_HAND_WRITTEN = $(foreach f,$(wildcard data/transducers/dot/*.dot), static/images/transducers/dot/$(subst .dot,.png,$(notdir $f)))
IMG_SET = $(foreach f,$(wildcard data/transducers/sets/*), static/images/transducers/sets/$(addsuffix .png,$(notdir $f)))
IMG_MAP = $(foreach f,$(wildcard data/transducers/maps/*), static/images/transducers/maps/$(addsuffix .png,$(notdir $f)))

code-transducers:
	./scripts/rust-from-blog content/post/transducers.md
	cargo update --manifest-path ./code/transducers/Cargo.toml
	cargo build --release --manifest-path ./code/transducers/Cargo.toml

img-transducers: $(IMG_HAND_WRITTEN) $(IMG_SET) $(IMG_MAP)

static/images/transducers/dot/%.png: data/transducers/dot/%.dot
	mkdir -p $(dir $@)
	dot -Tpng $< > $@

static/images/transducers/sets/%.png: tmp/data/transducers/sets/%.dot
	mkdir -p $(dir $@)
	dot -Tpng $< > $@

tmp/data/transducers/sets/%.dot: tmp/data/transducers/sets/%.fst
	mkdir -p $(dir $@)
	fst dot --state-names $< > $@

tmp/data/transducers/sets/%.fst: data/transducers/sets/%
	mkdir -p $(dir $@)
	fst set --sorted $< $@

static/images/transducers/maps/%.png: tmp/data/transducers/maps/%.dot
	mkdir -p $(dir $@)
	dot -Tpng $< > $@

tmp/data/transducers/maps/%.dot: tmp/data/transducers/maps/%.fst
	mkdir -p $(dir $@)
	fst dot --state-names $< > $@

tmp/data/transducers/maps/%.fst: data/transducers/maps/%
	mkdir -p $(dir $@)
	fst map --sorted $< $@

push:
	git push origin master
	git push github master

.PHONY: code
