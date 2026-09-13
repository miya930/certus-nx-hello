-- シミュレーション用のモデル。
-- Certus-NX High-Speed I/O Interface Technical Note には、Q0 がクロックの立ち上がり、
-- Q1 が立ち下がりのデータであることだけが書かれているため、それぞれのエッジで取り込むものとした。

library ieee;
use     ieee.std_logic_1164.all;

entity IDDRX1 is
    port (
    D       : in  std_logic;
    SCLK    : in  std_logic;
    RST     : in  std_logic;
    Q0      : out std_logic := '0';
    Q1      : out std_logic := '0');
end IDDRX1;

architecture sim of IDDRX1 is

begin

p_iddr : process(SCLK, RST)
begin
    if RST = '1' then
        Q0 <= '0';
        Q1 <= '0';
    elsif rising_edge(SCLK) then
        Q0 <= D;
    elsif falling_edge(SCLK) then
        Q1 <= D;
    end if;
end process;

end sim;
