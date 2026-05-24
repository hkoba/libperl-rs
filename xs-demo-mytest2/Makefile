MODNAME = Mytest2
WORKSPACE = ..
TARGET_DIR = $(WORKSPACE)/target/debug
SO_SRC = $(TARGET_DIR)/lib$(MODNAME).so

ARCH_DIR = blib/arch/auto/$(MODNAME)
LIB_DIR  = blib/lib

.PHONY: all build install test clean

all: install

build:
	cargo build -p $(MODNAME)

$(SO_SRC): build

$(ARCH_DIR)/$(MODNAME).so: $(SO_SRC)
	mkdir -p $(ARCH_DIR)
	cp -v $(SO_SRC) $@

$(LIB_DIR)/$(MODNAME).pm: perllib/$(MODNAME).pm
	mkdir -p $(LIB_DIR)
	cp -v perllib/$(MODNAME).pm $@

install: $(ARCH_DIR)/$(MODNAME).so $(LIB_DIR)/$(MODNAME).pm

test: install
	prove -Iblib/lib -Iblib/arch t/

clean:
	rm -rf blib
