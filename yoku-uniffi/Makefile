PKG_NAME ?= YokuUniffi
PLATFORMS ?= ios
BUILD_MODE ?= debug

CARGO ?= cargo

ifeq ($(BUILD_MODE),release)
	CARGO_BUILD_FLAG = --release
else
	CARGO_BUILD_FLAG =
endif

.PHONY: all YokuUniffi clean help

all: YokuUniffi
YokuUniffi:
	@$(CARGO) swift package \
		-p $(PLATFORMS) \
		-n $(PKG_NAME) \
		$(CARGO_BUILD_FLAG)

clean:
	@$(CARGO) clean
	@rm -rf YokuUniffi
