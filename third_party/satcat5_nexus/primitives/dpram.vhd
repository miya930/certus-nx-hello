library ieee;
use     ieee.numeric_std.all;
use     ieee.std_logic_1164.all;

entity dpram is
    generic (
    AWIDTH  : positive;
    DWIDTH  : positive;
    SIMTEST : boolean := false;
    TRIPORT : boolean := false);
    port (
    wr_clk  : in  std_logic;
    wr_addr : in  unsigned(AWIDTH - 1 downto 0);
    wr_en   : in  std_logic;
    wr_val  : in  std_logic_vector(DWIDTH - 1 downto 0);
    wr_rval : out std_logic_vector(DWIDTH - 1 downto 0);
    rd_clk  : in  std_logic;
    rd_addr : in  unsigned(AWIDTH - 1 downto 0);
    rd_en   : in  std_logic := '1';
    rd_val  : out std_logic_vector(DWIDTH - 1 downto 0));
end dpram;

architecture nexus of dpram is

subtype word_t is std_logic_vector(DWIDTH - 1 downto 0);
type ram_t is array(0 to 2 ** AWIDTH - 1) of word_t;

signal ram      : ram_t := (others => (others => '0'));
signal ram_tri  : ram_t := (others => (others => '0'));

begin

-- yosys がメモリとして推論できるよう、読み出しを同期にした基本形で書く。
p_ram : process(wr_clk, rd_clk)
begin
    if rising_edge(wr_clk) then
        if (wr_en = '1') then
            ram(to_integer(wr_addr)) <= wr_val;
        end if;
    end if;

    if rising_edge(rd_clk) then
        if (rd_en = '1') then
            rd_val <= ram(to_integer(rd_addr));
        end if;
    end if;
end process;

-- 書き込みアドレスの値を読み出す wr_rval は、同じ内容を持つ 2 つ目のメモリで作る。
gen_triport : if TRIPORT generate
    p_tri : process(wr_clk)
    begin
        if rising_edge(wr_clk) then
            if (wr_en = '1') then
                ram_tri(to_integer(wr_addr)) <= wr_val;
            end if;
            wr_rval <= ram_tri(to_integer(wr_addr));
        end if;
    end process;
end generate;

gen_twoport : if not TRIPORT generate
    wr_rval <= (others => '0');
end generate;

end nexus;
