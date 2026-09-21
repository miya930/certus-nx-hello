library ieee;
use ieee.std_logic_1164.all;

-- 命令メモリの先頭が 0 のあいだは待ち、JTAG から書き込まれたら先頭へ飛ぶ。
-- ブートローダの代わりにこの ROM を置く理由は、同じフォルダの README.md に書いた。
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
