FEATURES := $(CHIP)
BLOBS := --blob "1,../embassy/cyw43-firmware/43439A0.bin,2" --blob "2,../embassy/cyw43-firmware/43439A0_clm.bin,2" --blob "3,../embassy/cyw43-firmware/nvram_rp2040.bin,2"

mcaux-app: \
		nodist/utility.ihex \
		dist/release.elf \
		dist/release.bin \
		dist/stem.elf \
		dist/loader.elf \
		dist/release-bin-version \
		dist/release-bin-size \
		dist/release-bin-active-addr \
		dist/release-bin-dfu-addr \
		dist/release-bin-git-ref \
		dist/release-bin-git-status

# release.elf and stem.elf build ONCE ONLY: do "make clean && make" to guarantee freshness
dist/release.elf:
	mkdir -p dist
	cargo build --release --target $(TARGET) --features "$(FEATURES)"
	mv target/$(TARGET)/release/mcaux-app $@

dist/stem.elf:
	mkdir -p dist
	cargo build --release --target $(TARGET) --features "$(FEATURES),stem"
	mv target/$(TARGET)/release/mcaux-app $@

dist/loader.elf:
	mkdir -p dist
	cargo build --manifest-path ../mcaux-boot/Cargo.toml --release --target $(TARGET) --features "$(FEATURES)"
	mv ../mcaux-boot/target/$(TARGET)/release/mcaux-boot $@

dist/release.bin: dist/release.elf
	arm-none-eabi-objcopy -O binary $< $@

dist/release-bin-version: dist/release.bin
	cargo metadata --format-version 1 --no-deps|jq -r '.packages[0].version' > $@

dist/release-bin-size: dist/release.bin
	ls -l $@ | awk '{print $5}' > $@

dist/release-bin-git-status dist/release-bin-git-ref: dist/release.bin
	git status --porcelain > dist/release-bin-git-status
	if [ -f $< ] && [ -s $< ]; then rm -f dist/release-bin-git-ref ; \
		else git rev-parse --short HEAD > dist/release-bin-git-ref ; \
		fi

# Inconsequential whether these files exist
.PHONY: dist/release-bin-git-ref

dist/release-bin-active-addr dist/release-bin-dfu-addr dist/release-bin-utility-addr: memory-rp235xa.x
	mkdir -p dist
	awk '/^[ \t]*FLASH[ \t]*/ {print $$5}' $< > dist/release-bin-active-addr
	awk '/^[ \t]*DFU[ \t]*/ {print $$5}' $< > dist/release-bin-dfu-addr
	awk '/^[ \t]*UTILITY[ \t]*/ {print $$5}' $< > dist/release-bin-utility-addr

nodist/utility.ihex: dist/release-bin-utility-addr
	mkdir -p nodist
	if [ "${AP0}" != "" ]; then exit 0; else echo "Set at least AP0 and PW0" && exit 1 ; fi
	if [ "${PW0}" != "" ]; then exit 0; else echo "Set at least AP0 and PW0" && exit 1 ; fi
	if [ "${DFU0}" != "" ];then exit 0; else  echo "Set at least DFU0 update url prefix" && exit 1 ; fi
	cargo utility-section --load-address "`cat $<`" $(BLOBS) \
		--string "DFU0=${DFU0}" --string "DFU1=${DFU1}" --string "DFU2=${DFU2}" \
		--string "AP0=${AP0}" --string "PW0=${PW0}" \
		--string "AP1=${AP1}" --string "PW1=${PW1}" \
		--string "AP2=${AP2}" --string "PW2=${PW2}" \
		--string "AP3=${AP3}" --string "PW3=${PW3}" \
		--string "AP4=${AP4}" --string "PW4=${PW4}"
	mv utility.bin nodist
	mv utility.ihex nodist

clean:
	-rm -rf dist nodist
	-cargo clean
	-cargo clean --manifest-path ../mcaux-boot/Cargo.toml
