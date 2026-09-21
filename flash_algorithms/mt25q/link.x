/* 書き込みプログラムは、命令メモリを書き換えないようデータメモリに置く。
 * probe-rs は、このアドレスの直前の 8 バイトに、戻り先の ebreak を置く。 */
ALGO_PLACEMENT_START_ADDRESS = 0x80000020;

/* 巻き戻しの表は使わないため、読み込む中身に入れない。 */
SECTIONS
{
  /DISCARD/ : { *(.eh_frame .eh_frame_hdr) }
}
