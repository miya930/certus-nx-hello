-- SatCat5 の ice40_sync.vhd を Nexus でもそのまま使うため、
-- iCE40 の SB_DFFR と同じ端子を持つフリップフロップを Nexus のプリミティブで用意する。
-- 同期回路のフリップフロップが最適化で消えないよう、推論ではなくプリミティブを直接置く。

library ieee;
use     ieee.std_logic_1164.all;

entity SB_DFFR is
    port (
    D : in  std_logic;
    Q : out std_logic;
    C : in  std_logic;
    R : in  std_logic);
end SB_DFFR;

architecture nexus of SB_DFFR is

component FD1P3DX
    port (
    D  : in  std_logic;
    CK : in  std_logic;
    SP : in  std_logic;
    CD : in  std_logic;
    Q  : out std_logic);
end component;

begin

u_ff : FD1P3DX
    port map(
    D  => D,
    CK => C,
    SP => '1',
    CD => R,
    Q  => Q);

end nexus;
