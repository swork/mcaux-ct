MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes (0x400, so 0x1000 is 4K and 0x10000 is 64K) */
  BOOT2                             : ORIGIN = 0x10000000, LENGTH = 0x100
  FLASH                             : ORIGIN = 0x10000100, LENGTH = 64K - 0x100
  BOOTLOADER_STATE                  : ORIGIN = 0x10010000, LENGTH = 4K
  ACTIVE                            : ORIGIN = 0x10011000, LENGTH = 512K
  DFU                               : ORIGIN = 0x10091000, LENGTH = 516K

  /* Pick one of the two options for RAM layout     */

  /* OPTION A: Use all RAM banks as one big block   */
  /* Reasonable, unless you are doing something     */
  /* really particular with DMA or other concurrent */
  /* access that would benefit from striping        */
  RAM   : ORIGIN = 0x20000000, LENGTH = 264K

  /* OPTION B: Keep the unstriped sections separate */
  /* RAM: ORIGIN = 0x20000000, LENGTH = 256K        */
  /* SCRATCH_A: ORIGIN = 0x20040000, LENGTH = 4K    */
  /* SCRATCH_B: ORIGIN = 0x20041000, LENGTH = 4K    */
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(BOOT2);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(BOOT2);

__bootloader_active_start = ORIGIN(ACTIVE) - ORIGIN(BOOT2);
__bootloader_active_end = ORIGIN(ACTIVE) + LENGTH(ACTIVE) - ORIGIN(BOOT2);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(BOOT2);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(BOOT2);
