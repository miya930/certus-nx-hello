library ieee;
use ieee.std_logic_1164.all;

-- NEORV32 の内蔵ブートローダの代わりに、起動 ROM に置く。
-- 命令メモリは RAM で初期値を持たず、FPGA のコンフィグ直後は 0 で埋まっている。
-- 0 は不正命令で、コアは例外を繰り返し、デバッガが 1 命令進める操作を終えられない。
-- そこで、命令メモリの先頭が 0 のあいだはこの ROM の中で待ち、JTAG から書き込まれたら先頭へ飛ぶ。
package neorv32_bootrom_image is

type rom_t is array (0 to 3) of std_ulogic_vector(31 downto 0);
constant image_size_c : natural := 12;
constant image_data_c : rom_t := (
x"00002283", -- lw   t0, 0(zero)
x"fe028ee3", -- beqz t0, -4
x"00000067", -- jr   zero
others => (others => '0')
);

end package;
