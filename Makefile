.PHONY: nightly build test watch lint

ifeq ($(OS),Windows_NT)
    MACHINE = WIN32
    ifeq ($(PROCESSOR_ARCHITECTURE),AMD64)
        MACHINE += AMD64
    endif
else
    UNAME_S := $(shell uname -s)
    ifeq ($(UNAME_S),Linux)
        MACHINE = LINUX
    endif
    ifeq ($(UNAME_S),Darwin)
        MACHINE = OSX
    endif
    UNAME_P := $(shell uname -p)
    ifeq ($(UNAME_P),x86_64)
        MACHINE += AMD64
    endif
    ifneq ($(filter arm%,$(UNAME_P)),)
        MACHINE += ARM
    endif
endif

TARGET = linux-x86_64
LIB_NAME = libcompass
LIB_EXTENSION = so

ifeq ($(MACHINE),WIN32 AMD64)
	TARGET = windows-x86_64
	LIB_NAME = compass
    LIB_EXTENSION = dll
    COPY_COMMAND = copy
else ifeq ($(MACHINE),LINUX AMD64)
	TARGET = linux-x86_64
	LIB_NAME = libcompass
    LIB_EXTENSION = so
    COPY_COMMAND = cp -f
else ifeq ($(MACHINE),OSX AMD64)
	TARGET = mac-x86_64
	LIB_NAME = libcompass
    LIB_EXTENSION = dylib
    COPY_COMMAND = cp -f
else ifeq ($(MACHINE),OSX ARM)
	TARGET = mac-aarch64
	LIB_NAME = libcompass
    LIB_EXTENSION = dylib
    COPY_COMMAND = cp -f
else
    COPY_COMMAND = cp -f
endif

nightly:
	curl -L -o ./lua/compass.$(LIB_EXTENSION) https://github.com/myypo/compass.nvim/releases/download/nightly/$(TARGET).$(LIB_EXTENSION)

build:
	cargo build --release
	$(COPY_COMMAND) ./target/release/$(LIB_NAME).$(LIB_EXTENSION) ./lua/compass.$(LIB_EXTENSION)

test:
	cargo build
	$(COPY_COMMAND) ./target/debug/$(LIB_NAME).$(LIB_EXTENSION) ./lua/compass.$(LIB_EXTENSION)
	cargo test

watch:
	cargo-watch -i lua -- make test

lint:
	cargo clippy -- -D warnings
