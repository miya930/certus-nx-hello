-- シミュレーション用のモデル。
-- Certus-NX High-Speed I/O Interface Technical Note の DELAYA の属性の表に合わせ、
-- COARSE_DELAY と DEL_VALUE から求めた時間だけ入力を遅らせる。

library ieee;
use     ieee.std_logic_1164.all;

entity DELAYA is
    generic (
    DEL_MODE            : string := "USER_DEFINED";
    DEL_VALUE           : natural := 0;
    COARSE_DELAY_MODE   : string := "STATIC";
    COARSE_DELAY        : string := "0NS");
    port (
    A           : in  std_logic;
    Z           : out std_logic);
end DELAYA;

architecture sim of DELAYA is

constant FINE_STEP : time := 12500 fs;

function coarse_time return time is
begin
    if COARSE_DELAY = "0P8NS" then
        return 800 ps;
    elsif COARSE_DELAY = "1P6NS" then
        return 1600 ps;
    else
        return 0 ps;
    end if;
end function;

begin

Z <= transport A after coarse_time + DEL_VALUE * FINE_STEP;

end sim;
