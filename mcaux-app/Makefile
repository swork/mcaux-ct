#! /usr/bin/env make -f

TARGET := thumbv8m.main-none-eabihf
# TARGET := thumbv6m-none-eabi
FEATURE :=
# FEATURE := rp2040
DFU_FULL := target/$(TARGET)/debug/mcaux-app
BLOBS := --blob "1,../embassy/cyw43-firmware/43439A0.bin,2" --blob "2,../embassy/cyw43-firmware/43439A0_clm.bin,2" --blob "3,../embassy/cyw43-firmware/nvram_rp2040.bin,2"
LOAD_ADDRESS := 0x10222000  # coordinate with memory.x - this is FLASH location
UTIL_ARGS := --load-address $(LOAD_ADDRESS) $(BLOBS)

mcaux-rp235x: utility.ihex stem.elf $(DFU_FULL)

clean:
	-rm -f utility.ihex utility.bin stem.elf
	-cargo clean

utility.ihex: Makefile
	-[ "${AP0}" == "" ] && echo "Set at least AP0 and PW0" && exit 1
	-[ "${PW0}" == "" ] && echo "Set at least AP0 and PW0" && exit 1
	-[ "${URL}" == "" ] && echo "Set DFU update URL as 'URL'" && exit 1
	cargo utility-section $(UTIL_ARGS) --string "DFU=${URL}" --string "AP0=${AP0}" --string "PW0=${PW0}" --string "AP1=${AP1}" --string "PW1=${PW1}" --string "AP2=${AP2}" --string "PW2=${PW2}" --string "AP3=${AP3}" --string "PW3=${PW3}" 

stem.elf:
	cargo build --features stem
	mv $(DFU_FULL) ./stem.elf

$(DFU_FULL):
	cargo build --target $(TARGET) --features "$(FEATURE)"
