-- SatCat5 のプラットフォームごとの定数。

library ieee;
use     ieee.std_logic_1164.all;
use     work.common_primitives.all;

package body common_primitives is
    -- MAC テーブルの TCAM は、LUT_WIDTH を 8 にすると学習が続いたときに同じアドレスを重複して登録したため、
    -- SatCat5 のテストで確認されている 6 にする。
    constant PREFER_DPRAM_AWIDTH : positive := 6;

    -- 1 ビット単位で書き込むと TCAM のメモリがビットごとに分かれ、LFD2NX-40 の LUT に収まらないため、ワード単位で書き込む。
    constant PREFER_DPRAM_ONEBIT : boolean := false;

    -- シフトレジスタ方式の FIFO はフリップフロップで作られて LFD2NX-40 に収まらないため、メモリ方式にする。
    -- メモリ方式は出力が 1 サイクル余計に遅れ、fifo_repack のメタデータがずれる。
    -- fifo_repack は VLAN の受信処理で使うため、VLAN を有効にするときは fifo_repack を直す必要がある。
    constant PREFER_FIFO_SREG    : boolean := false;

    constant PREFER_SPI_SYNC     : boolean := false;

    -- PTP を使わないため、Vernier の構成は作らない。
    function create_vernier_config(
        input_hz    : natural;
        sync_tau_ms : real := VERNIER_DEFAULT_TAU_MS;
        sync_aux_en : boolean := VERNIER_DEFAULT_AUX_EN;
        sync_frq_en : boolean := VERNIER_DEFAULT_FRQ_EN)
    return vernier_config is
    begin
        return VERNIER_DISABLED;
    end function;
end package body;
