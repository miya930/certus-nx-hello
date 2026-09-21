library ieee;
use     ieee.std_logic_1164.all;

entity clk_input is
    generic (
    CLKIN_MHZ   : real;
    GLOBAL_BUFF : boolean := false;
    DESKEW_EN   : boolean := false;
    DELAY_NSEC  : real := -1.0);
    port (
    reset_p : in  std_logic;
    shdn_p  : in  std_logic := '0';
    clk_pin : in  std_logic;
    clk_out : out std_logic);
end clk_input;

architecture nexus of clk_input is

begin

gen_delay : if DELAY_NSEC >= 0.0 generate
    assert false report "DELAY_NSEC is not implemented" severity error;
end generate;

gen_deskew : if DESKEW_EN generate
    assert false report "DESKEW_EN is not implemented" severity error;
end generate;

-- クロック網への割り当ては nextpnr-nexus が行うため、バッファは置かない。
clk_out <= clk_pin;

end nexus;
