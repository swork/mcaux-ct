#! /usr/bin/env make -f

TARGET_RP235x := thumbv8m.main-none-eabihf
DFU_RP235x    := mcaux-rp235x
DFU_RP235x_FULL := target/$(TARGET_RP235x)/debug/$(DFU_RP235x)
BLOBS := --blob "1,../embassy/cyw43-firmware/43439A0.bin" --blob "2,../embassy/cyw43-firmware/43439A0_clm.bin" --blob "3,../embassy/cyw43-firmware/nvram_rp2040.bin"
LOAD_ADDRESS := 0x10222000  # coordinate with memory.x - this is FLASH location
UTIL_ARGS := --load-address $(LOAD_ADDRESS) $(BLOBS)

mcaux-rp235x: utility.ihex stem.elf $(DFU_RP235x_FULL)

clean:
	-rm -f utility.ihex stem.elf
	-cargo clean

utility.ihex:
	-[ "${AP0}" == "" ] && echo "Set at least AP0 and PW0" && exit 1
	-[ "${PW0}" == "" ] && echo "Set at least AP0 and PW0" && exit 1
	-[ "${DFU}" == "" ] && echo "Set update URL as 'DFU'" && exit 1
	cargo utility-section $(UTIL_ARGS) --string "DFU=${DFU}" --string "AP0=${AP0}" --string "PW0=${PW0}" --string "AP1=${AP1}" --string "PW1=${PW1}" --string "AP2=${AP2}" --string "PW2=${PW2}" --string "AP3=${AP3}" --string "PW3=${PW3}"  --string "AP4=${AP4}" --string "PW4=${PW4}"

stem.elf:
	cargo build --features stem
	mv $(DFU_RP235x_FULL) ./stem.elf

$(DFU_RP235x_FULL):
	cargo build
