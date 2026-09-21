/* 起動 ROM は NEORV32 の BOOTROM の位置に置き、スタックはデータメモリの先頭の 1 KB に置く。 */
MEMORY
{
  BOOTROM : ORIGIN = 0xFFE00000, LENGTH = 4K
  STACK   : ORIGIN = 0x80000000, LENGTH = 1K
}

ENTRY(_start)

SECTIONS
{
  .text :
  {
    KEEP(*(.text.start))
    *(.text .text.*)
    *(.rodata .rodata.* .srodata .srodata.*)
  } > BOOTROM

  /* ROM から初期値を写す処理を持たないため、初期値のある静的変数は持たない。 */
  .data : { *(.data .data.* .sdata .sdata.*) } > BOOTROM
  /* 0 で埋める処理も持たないため、.bss は PAC の steal が書くだけの印に限る。 */
  .bss (NOLOAD) : { *(.bss .bss.* .sbss .sbss.*) } > STACK

  /DISCARD/ : { *(.eh_frame .eh_frame_hdr) }
}

ASSERT(SIZEOF(.data) == 0, "the boot ROM cannot initialize static variables")

_stack_top = ORIGIN(STACK) + LENGTH(STACK);
