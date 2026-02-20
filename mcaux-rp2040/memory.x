MEMORY
{
BOOT2   : ORIGIN = 0x10000000, LENGTH = 0x100
FLASH   : ORIGIN = 0x10000100, LENGTH = 1792K - 0x100
UTILITY : ORIGIN = 0x101C0000, LENGTH = 256K
RAM     : ORIGIN = 0x20000000, LENGTH = 264K
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
.bi_entries : ALIGN(4)
{
__bi_entries_start = .;
KEEP(*(.bi_entries));
. = ALIGN(4);
__bi_entries_end = .;
} > FLASH
} INSERT AFTER .text;

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

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);

__utility_start = ORIGIN(UTILITY);
