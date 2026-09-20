library ieee;
use     ieee.std_logic_1164.all;

entity ddr_output is
    port (
    d_re    : in  std_logic;
    d_fe    : in  std_logic;
    clk     : in  std_logic;
    q_pin   : out std_logic);
end ddr_output;

architecture nexus of ddr_output is

component ODDRX1
    port (
    D0   : in  std_logic;
    D1   : in  std_logic;
    SCLK : in  std_logic;
    RST  : in  std_logic;
    Q    : out std_logic);
end component;

begin

u_oddr : ODDRX1
    port map(
    D0   => d_re,
    D1   => d_fe,
    SCLK => clk,
    RST  => '0',
    Q    => q_pin);

end nexus;
