TARGET = aarch64-unknown-none
BUILD = release
BUILD_DIR = target/$(TARGET)/$(BUILD)

OBJCOPY := $(shell find $(shell rustc --print sysroot) -name llvm-objcopy)

all: clean kernel

$(BUILD_DIR)/zinc64-raspi:
	cargo xbuild -p zinc64-raspi --target=$(TARGET) --release 

kernel: $(BUILD_DIR)/zinc64-raspi
	$(OBJCOPY) --strip-all -O binary $< $(BUILD_DIR)/zinc64-raspi.img

run: kernel
	qemu-system-aarch64 -M raspi3 -nographic -semihosting-config enable=on,target=native -kernel $(BUILD_DIR)/zinc64-raspi.img

clean:
	cargo clean
