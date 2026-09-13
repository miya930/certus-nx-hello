-- シミュレーション用のモデル。yosys の Nexus 向けシミュレーションモデルと同じ動作にする。

library ieee;
use     ieee.std_logic_1164.all;

entity FD1P3DX is
    port (
    D  : in  std_logic;
    CK : in  std_logic;
    SP : in  std_logic;
    CD : in  std_logic;
    Q  : out std_logic := '0');
end FD1P3DX;

architecture sim of FD1P3DX is

begin

p_ff : process(CK, CD)
begin
    if CD = '1' then
        Q <= '0';
    elsif rising_edge(CK) then
        if SP = '1' then
            Q <= D;
        end if;
    end if;
end process;

end sim;
