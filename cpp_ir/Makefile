.PHONY: all clean test verify fmt

SRCS := $(wildcard src/*.cc)
BINS := $(patsubst src/%.cc, autogen/%.bin, $(SRCS))

INPUT := $(wildcard *.mrn)
JSONS := $(patsubst %.mrn, autogen/%.mrn.json, $(INPUT))
TESTS := $(patsubst %.mrn, autogen/%.mrn.test.h, $(INPUT))

test: all
	@./autogen/output_schema.bin >autogen/ir_schema.md
	@./autogen/output_schema.bin --rust >autogen/ir_schema.rs
	./run_tests.sh

verify:
	./verify.sh

fmt:
	for i in $$(find src/ -name '*.cc'); do clang-format -i "$$i" ; done
	for i in $$(find src/ -name '*.h'); do clang-format -i "$$i" ; done

all: $(BINS) $(JSONS) $(TESTS)

# NOTE(dkorolev): This will also delete the `autogen/*.mrn.json` files, but they are re-generated easily.
clean:
	@rm -rf autogen

autogen:
	@mkdir -p autogen

autogen/%.bin: src/%.cc src/*.h src/boilerplate/*.h
	@mkdir -p autogen
	@[ -f current/current.h ] || git clone --depth 1 https://github.com/C5T/Current current
	g++ $< -o $@

autogen/%.mrn.json: %.mrn
	@mkdir -p autogen
	@[ -f current/current.h ] || git clone --depth 1 https://github.com/C5T/Current current
	@./mrn2ir.sh $<

autogen/%.mrn.test.h: %.mrn ./autogen/gen_test.bin
	@./ir2test.sh $< >$@
