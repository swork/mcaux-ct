FEATURES := $(CHIP)
BLOBS := --blob "1,../embassy/cyw43-firmware/43439A0.bin,2" --blob "2,../embassy/cyw43-firmware/43439A0_clm.bin,2" --blob "3,../embassy/cyw43-firmware/nvram_rp2040.bin,2"
V = "`cat b/public/release.bin.version`"

.PHONY: tarballz

tarballz: b/archive/mcaux_latest_$(BOARD).tar.gz b/archive/mcaux_$(V)_$(BOARD).tar.gz

b/archive/mcaux_latest_$(BOARD).tar.gz b/archive/mcaux_$(V)_$(BOARD).tar.gz: \
		b/private/utility.ihex \
		b/public/release.elf \
		b/public/release.bin \
		b/public/stem.elf \
		b/public/loader.elf \
		b/public/release.bin.version \
		b/public/release.bin.sha256 \
		b/public/release.bin.active-addr \
		b/public/release.bin.dfu-addr \
		b/public/release.bin.utility-addr \
		b/public/release.bin.git-ref \
		b/public/release.bin.git-status
	mkdir -p b/archive
	mkdir -p b/tar/mcaux/$(BOARD)/latest && cp b/public/* b/tar/mcaux/$(BOARD)/latest && tar -C b/tar -czf b/archive/mcaux_latest_$(BOARD).tar.gz . && rm -rf b/tar
	-mkdir -p b/tar/mcaux/$(BOARD)/$(V) && cp b/public/* b/tar/mcaux/$(BOARD)/$(V) && tar -C b/tar -czf b/archive/mcaux_$(V)_$(BOARD).tar.gz . && rm -rf b/tar

# release.elf and stem.elf build ONCE ONLY: do "make clean && make" to guarantee freshness
b/public/release.elf:
	mkdir -p b/public
	DEFMT_LOG=embassy_boot=trace,embassy_boot_rp=trace,embassy_rp::flash=trace,info cargo build --release --target $(TARGET) --features "$(FEATURES)"
	cp target/$(TARGET)/release/mcaux $@

# Don't let make build these in parallel
b/public/stem.elf: b/public/release.elf
	mkdir -p b/public
	DEFMT_LOG=embassy_boot=trace,embassy_boot_rp=trace,embassy_rp::flash=trace,info cargo build --release --target $(TARGET) --features "$(FEATURES),stem"
	cp target/$(TARGET)/release/mcaux $@

b/public/loader.elf:
	mkdir -p b/public
	DEFMT_LOG=embassy_boot=trace,embassy_boot_rp=trace,embassy_rp::flash=trace,info cargo build --manifest-path ../mcaux-boot/Cargo.toml --release --target $(TARGET) --features "$(FEATURES),defmt,blink"
	cp ../mcaux-boot/target/$(TARGET)/release/mcaux-boot $@

b/public/release.bin: b/public/release.elf
	arm-none-eabi-objcopy -O binary $< $@

b/public/release.bin.version: b/public/release.bin
	cargo metadata --format-version 1 --no-deps|jq -r '.packages[0].version' > $@

b/public/release.bin.sha256: b/public/release.bin
	sha256 -q $< > $@

b/public/release.bin.git-status b/public/release.bin.git-ref: b/public/release.bin
	git status --porcelain > b/public/release.bin.git-status
	if [ -f b/public/release.bin.git-status ] && [ -s b/public/release.bin.git-status ]; then rm -f b/public/release.bin.git-ref ; \
		else git rev-parse --short HEAD > b/public/release.bin.git-ref ; \
		fi

# Inconsequential whether this file exists
.PHONY: b/public/release.bin.git-ref

b/public/release.bin.active-addr b/public/release.bin.dfu-addr b/public/release.bin.utility-addr: memory-rp235xa.x
	mkdir -p b/public
	awk '/^[ \t]*FLASH[ \t]*/ {print $$5}' $< > b/public/release.bin.active-addr
	awk '/^[ \t]*DFU[ \t]*/ {print $$5}' $< > b/public/release.bin.dfu-addr
	awk '/^[ \t]*UTILITY[ \t]*/ {print $$5}' $< > b/public/release.bin.utility-addr

b/private/utility.ihex: b/public/release.bin.utility-addr
	mkdir -p b/private
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
	mv utility.bin b/private
	mv utility.ihex b/private

clean:
	-rm -rf b
	-cargo clean
	-cargo clean --manifest-path ../mcaux-boot/Cargo.toml
