TARGET = aarch64-unknown-none
BUILD = release
BUILD_DIR = target/$(TARGET)/$(BUILD)

OBJCOPY := $(shell find $(shell rustc --print sysroot) -name llvm-objcopy)
OBJDUMP := $(shell find $(shell rustc --print sysroot) -name llvm-objdump)

SOURCES = $(wildcard **/**/*.rs) $(wildcard **/**/**/*.rs) rpi64.ld

all: clean kernel

$(BUILD_DIR)/zinc64-rpi: $(SOURCES)
	cargo xbuild -p zinc64-rpi --target=$(TARGET) --release

$(BUILD_DIR)/kernel8.img: $(BUILD_DIR)/zinc64-rpi
	$(OBJCOPY) --strip-all -O binary $< $(BUILD_DIR)/kernel8.img

kernel: $(BUILD_DIR)/kernel8.img

run: kernel
	qemu-system-aarch64 -M raspi3 -drive file=sd.img,if=sd,format=raw -serial stdio -kernel $(BUILD_DIR)/kernel8.img

sd: $(BUILD_DIR)/kernel8.img
	cp $< /run/media/raytracer/rpi/

clean:
	cargo clean
