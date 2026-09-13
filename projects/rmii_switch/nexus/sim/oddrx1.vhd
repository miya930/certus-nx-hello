-- シミュレーション用のモデル。
-- Certus-NX High-Speed I/O Interface Technical Note には、D0 を先に、D1 を後に出力し、
-- SCLK の両エッジで出力が変わることだけが書かれている。
-- 取り込みのタイミングは記載がないため、SCLK の立ち上がりで D0 と D1 を取り込むものとした。

library ieee;
use     ieee.std_logic_1164.all;

entity ODDRX1 is
    port (
    D0   : in  std_logic;
    D1   : in  std_logic;
    SCLK : in  std_logic;
    RST  : in  std_logic;
    Q    : out std_logic := '0');
end ODDRX1;

architecture sim of ODDRX1 is

signal d1_hold : std_logic := '0';

begin

p_oddr : process(SCLK, RST)
begin
    if RST = '1' then
        Q       <= '0';
        d1_hold <= '0';
    elsif rising_edge(SCLK) then
        Q       <= D0;
        d1_hold <= D1;
    elsif falling_edge(SCLK) then
        Q       <= d1_hold;
    end if;
end process;

end sim;
