/* These settings must coordinate with ../mcaux-rp2040/memory.x:
  - ACTIVE here is FLASH there
 */

MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes (0x400, so 0x1000 is 4K and 0x10000 is 64K) */
  BOOT2                             : ORIGIN = 0x10000000, LENGTH = 0x100
  FLASH                             : ORIGIN = 0x10000100, LENGTH = 64K - 0x100
  BOOTLOADER_STATE                  : ORIGIN = 0x10010000, LENGTH = 4K
  ACTIVE                            : ORIGIN = 0x10011000, LENGTH = 512K
  DFU                               : ORIGIN = 0x10091000, LENGTH = 516K
  RAM   : ORIGIN = 0x20000000, LENGTH = 264K
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(BOOT2);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(BOOT2);

__bootloader_active_start = ORIGIN(ACTIVE) - ORIGIN(BOOT2);
__bootloader_active_end = ORIGIN(ACTIVE) + LENGTH(ACTIVE) - ORIGIN(BOOT2);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(BOOT2);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(BOOT2);
