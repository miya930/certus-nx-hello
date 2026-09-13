library ieee;
use     ieee.math_real.all;
use     ieee.std_logic_1164.all;

entity ddr_input is
    generic (
    DELAY_NSEC  : real := -1.0);
    port (
    d_pin   : in  std_logic;
    clk     : in  std_logic;
    q_re    : out std_logic;
    q_fe    : out std_logic);
end ddr_input;

architecture nexus of ddr_input is

-- DELAYA は、0.8 ns 刻みの粗い遅延と 12.5 ps 刻みの細かい遅延を足し合わせて使う。
constant COARSE_STEP_NSEC   : real := 0.8;
constant COARSE_STEP_MAX    : natural := 2;
constant FINE_STEP_NSEC     : real := 0.0125;
constant FINE_STEP_MAX      : natural := 127;

function coarse_steps return natural is
begin
    return integer(realmin(floor(DELAY_NSEC / COARSE_STEP_NSEC), real(COARSE_STEP_MAX)));
end function;

function fine_steps return natural is
    constant remain : real := DELAY_NSEC - real(coarse_steps) * COARSE_STEP_NSEC;
begin
    return integer(realmin(round(remain / FINE_STEP_NSEC), real(FINE_STEP_MAX)));
end function;

function coarse_name return string is
begin
    case coarse_steps is
        when 0      => return "0NS";
        when 1      => return "0P8NS";
        when others => return "1P6NS";
    end case;
end function;

component DELAYA
    generic (
    DEL_MODE            : string := "USER_DEFINED";
    DEL_VALUE           : natural := 0;
    COARSE_DELAY_MODE   : string := "STATIC";
    COARSE_DELAY        : string := "0NS");
    port (
    A           : in  std_logic;
    Z           : out std_logic);
end component;

component IDDRX1
    port (
    D       : in  std_logic;
    SCLK    : in  std_logic;
    RST     : in  std_logic;
    Q0      : out std_logic;
    Q1      : out std_logic);
end component;

signal d_dly : std_logic;

begin

-- nextpnr-nexus は遅延量を直接指定する USER_DEFINED だけに対応するため、DELAY_NSEC を段数に換算する。
-- 遅延量を動的に変える端子は、3.3 V の I/O セルに配線がなく配置配線が失敗するため、宣言しない。
gen_delay : if DELAY_NSEC >= 0.0 generate
    u_delay : DELAYA
        generic map(
        DEL_VALUE       => fine_steps,
        COARSE_DELAY    => coarse_name)
        port map(
        A           => d_pin,
        Z           => d_dly);
end generate;

gen_direct : if DELAY_NSEC < 0.0 generate
    d_dly <= d_pin;
end generate;

u_iddr : IDDRX1
    port map(
    D       => d_dly,
    SCLK    => clk,
    RST     => '0',
    Q0      => q_re,
    Q1      => q_fe);

end nexus;
