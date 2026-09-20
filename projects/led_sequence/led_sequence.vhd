library ieee;
use     ieee.numeric_std.all;
use     ieee.std_logic_1164.all;

entity led_sequence is
    generic (
    CLK_HZ      : positive := 25_000_000;
    STEP_MSEC   : positive := 100);
    port (
    system_25m_clk  : in  std_logic;
    pushbutton3     : in  std_logic;
    led             : out std_logic_vector(7 downto 0));
end led_sequence;

architecture rtl of led_sequence is

constant STEP_CYCLES    : positive := CLK_HZ / 1000 * STEP_MSEC;

signal reset_p  : std_logic;
signal count    : natural range 0 to STEP_CYCLES-1 := 0;
signal value    : unsigned(led'range) := (others => '0');

begin

-- 押しボタンは、押している間だけ 0 になる。
reset_p <= not pushbutton3;

-- 値は LED の数と同じ幅なので、最大値の次は桁が溢れて 0 に戻る。
p_step : process(system_25m_clk)
begin
    if rising_edge(system_25m_clk) then
        if reset_p = '1' then
            count <= 0;
            value <= (others => '0');
        elsif count = STEP_CYCLES-1 then
            count <= 0;
            value <= value + 1;
        else
            count <= count + 1;
        end if;
    end if;
end process;

-- LED は、出力を 0 にすると点灯する。
led <= not std_logic_vector(value);

end rtl;
