/* Settings here must coordinate with ../mcaux-rp2040-boot/memory.x:
  - LOADER here is FLASH there
  - FLASH here is ACTIVE there
 */

MEMORY
{
BOOT2            : ORIGIN = 0x10000000, LENGTH = 0x100
LOADER           : ORIGIN = 0x10000100, LENGTH = 64K - 0x100
BOOTLOADER_STATE : ORIGIN = 0x10010000, LENGTH = 4K
FLASH            : ORIGIN = 0x10011000, LENGTH = 512K
DFU              : ORIGIN = 0x10091000, LENGTH = 516K
UTILITY          : ORIGIN = 0x10112000, LENGTH = 256K
RAM              : ORIGIN = 0x20000000, LENGTH = 264K
}

SECTIONS {
.start_block : ALIGN(4)
{
__start_block_addr = .;
KEEP(*(.start_block));
KEEP(*(.boot_info));
} > FLASH
} INSERT AFTER .vector_table;

/* move .text to start /after/ the boot info */
_stext = ADDR(.start_block) + SIZEOF(.start_block);

SECTIONS {
.end_block : ALIGN(4)
{
__end_block_addr = .;
KEEP(*(.end_block));
} > FLASH
} INSERT AFTER .uninit;

SECTIONS {
.utility_block : ALIGN(4)
{
__utility_block = .;
KEEP(*(.utility_block));
} > UTILITY
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(BOOT2);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(BOOT2);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(BOOT2);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(BOOT2);

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);

__utility_start = ORIGIN(UTILITY);
