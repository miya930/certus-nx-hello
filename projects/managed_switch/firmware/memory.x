/* 位置と大きさは managed_switch.vhd の generic に合わせる。 */
MEMORY
{
  IMEM : ORIGIN = 0x00000000, LENGTH = 64K
  DMEM : ORIGIN = 0x80000000, LENGTH = 16K
}

REGION_ALIAS("REGION_TEXT",   IMEM);
REGION_ALIAS("REGION_RODATA", IMEM);
REGION_ALIAS("REGION_DATA",   DMEM);
REGION_ALIAS("REGION_BSS",    DMEM);
REGION_ALIAS("REGION_HEAP",   DMEM);
REGION_ALIAS("REGION_STACK",  DMEM);

/* パニックでは止まるだけで巻き戻さないため、巻き戻しの表は捨てて命令メモリを空ける。 */
SECTIONS
{
  /DISCARD/ : { *(.eh_frame) *(.eh_frame_hdr) }
}
