MEMORY
{
  ZERO             : ORIGIN = 0x10000000, LENGTH = 0
  BOOT2            : ORIGIN = 0x10000000, LENGTH = 0x100
  LOADER           : ORIGIN = 0x10000100, LENGTH = 128K - 0x100
  BOOTLOADER_STATE : ORIGIN = 0x10020000, LENGTH = 4K
  UTILITY          : ORIGIN = 0x10021000, LENGTH = 448K
  FLASH            : ORIGIN = 0x10091000, LENGTH = 732K
  DFU              : ORIGIN = 0x10148000, LENGTH = 736K
  RAM      : ORIGIN = 0x20000000, LENGTH = 264K
}

SECTIONS {
  .start_block : ALIGN(4)
  {
    __start_block_addr = .;
    KEEP(*(.start_block));
  } > FLASH
} INSERT AFTER .vector_table;

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
    __utility_block_addr = .;
    KEEP(*(.utility_block));
  } > UTILITY
}

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(ZERO);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(ZERO);

__bootloader_active_start = ORIGIN(FLASH) - ORIGIN(ZERO);
__bootloader_active_end = ORIGIN(FLASH) + LENGTH(FLASH) - ORIGIN(ZERO);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(ZERO);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(ZERO);

PROVIDE(utility_block_starts_here = __utility_block_addr - ORIGIN(ZERO));
PROVIDE(utility_block_ends_here = __utility_block_addr - ORIGIN(ZERO) + LENGTH(UTILITY));
