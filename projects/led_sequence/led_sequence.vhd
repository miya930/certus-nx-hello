library ieee;
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

constant LED_COUNT      : positive := 8;
constant STEP_CYCLES    : positive := CLK_HZ / 1000 * STEP_MSEC;

signal reset_p  : std_logic;
signal count    : natural range 0 to STEP_CYCLES-1 := 0;
signal index    : natural range 0 to LED_COUNT-1 := 0;

begin

-- 押しボタンは、押している間だけ 0 になる。
reset_p <= not pushbutton3;

p_step : process(system_25m_clk)
begin
    if rising_edge(system_25m_clk) then
        if reset_p = '1' then
            count <= 0;
            index <= 0;
        elsif count = STEP_CYCLES-1 then
            count <= 0;
            if index = LED_COUNT-1 then
                index <= 0;
            else
                index <= index + 1;
            end if;
        else
            count <= count + 1;
        end if;
    end if;
end process;

-- LED は、出力を 0 にすると点灯する。
p_led : process(index)
begin
    led <= (others => '1');
    led(index) <= '0';
end process;

end rtl;
