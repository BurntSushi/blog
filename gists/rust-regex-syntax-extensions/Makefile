RUSTC ?= rustc
RUST_PATH ?= -L ./build
BUILD_DIR ?= ./build
LIBS = $(foreach lib,$(wildcard lib/*.rs), $(BUILD_DIR)/.timestamp_lib_$(notdir $(lib:.rs=)))
GISTS_ID ?= 11161035
GISTS = $(wildcard *.rs)
GISTS_EXES = $(foreach gist,$(GISTS),$(BUILD_DIR)/$(gist:.rs=))
GISTS_UPLOADS = $(foreach gist,$(GISTS), $(BUILD_DIR)/.timestamp_gist_$(gist:.rs=))

GISTS_OTHER = $(wildcard other/*.rs)
GISTS_OTHER_UPLOADS = $(foreach gist,$(GISTS_OTHER), $(BUILD_DIR)/.timestamp_other_gist_$(notdir $(gist:.rs=)))

all: $(LIBS) $(GISTS_UPLOADS) $(GISTS_OTHER_UPLOADS)

$(BUILD_DIR)/.timestamp_lib_%: lib/%.rs
	$(RUSTC) $(RUST_PATH) --out-dir=$(BUILD_DIR) $<
	gist -f "$(subst /,-,$<)" -u $(GISTS_ID) $<
	touch $@

$(BUILD_DIR)/.timestamp_other_gist_%: other/%.rs
	gist -u $(GISTS_ID) other/$*.rs
	touch $@

$(BUILD_DIR)/.timestamp_gist_%: $(BUILD_DIR)/%
	gist -u $(GISTS_ID) $*.rs
	touch $@

$(BUILD_DIR)/%: %.rs $(LIBS)
	$(RUSTC) $(RUST_PATH) -o $@ $<
	./check $@ < $<

clean:
	rm -f $(BUILD_DIR)/.timestamp*
	rm -f $(BUILD_DIR)/*.so
	rm -f $(BUILD_DIR)/*.rlib

