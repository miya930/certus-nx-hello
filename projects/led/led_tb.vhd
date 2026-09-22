library ieee;
use     ieee.numeric_std.all;
use     ieee.std_logic_1164.all;

entity led_tb is
end led_tb;

architecture sim of led_tb is

-- 値を 1 増やす間隔が 10 サイクルになるようにして、短い時間で確認する。
constant CLK_HZ         : positive := 10_000;
constant STEP_MSEC      : positive := 1;
constant STEP_CYCLES    : positive := CLK_HZ / 1000 * STEP_MSEC;
constant CLK_PERIOD     : time := 1 sec / CLK_HZ;
constant LED_COUNT      : positive := 8;
constant VALUE_COUNT    : positive := 2 ** LED_COUNT;

-- LED は 0 で点灯するため、値を反転したものが出力になる。
function led_of(value : natural) return std_logic_vector is
begin
    return not std_logic_vector(to_unsigned(value, LED_COUNT));
end function;

signal clk          : std_logic := '0';
signal pushbutton3  : std_logic := '1';
signal led          : std_logic_vector(LED_COUNT - 1 downto 0);
signal test_done    : boolean := false;

begin

uut : entity work.led
    generic map(
    CLK_HZ      => CLK_HZ,
    STEP_MSEC   => STEP_MSEC)
    port map(
    system_25m_clk  => clk,
    pushbutton3     => pushbutton3,
    led             => led);

p_clk : process
begin
    while not test_done loop
        clk <= '0';
        wait for CLK_PERIOD / 2;
        clk <= '1';
        wait for CLK_PERIOD / 2;
    end loop;
    wait;
end process;

p_test : process
    variable previous   : std_logic_vector(LED_COUNT - 1 downto 0);
    variable cycles     : natural;

    -- 出力が落ち着いた時刻で見るため、立ち上がりから 1/4 周期ずらして観測する。
    procedure next_sample is
    begin
        wait until rising_edge(clk);
        wait for CLK_PERIOD / 4;
    end procedure;
begin
    -- 押しボタンを押した状態から始め、離した直後の状態を基準にする。
    pushbutton3 <= '0';
    for n in 1 to 3 loop
        next_sample;
    end loop;
    assert led = led_of(0)
        report "reset: value is not 0" severity error;
    pushbutton3 <= '1';

    -- 1 周ぶん数え、最大値の次に 0 へ戻ってからも続くことを確認する。
    for step in 1 to VALUE_COUNT + 1 loop
        previous := led;
        cycles := 0;
        loop
            next_sample;
            cycles := cycles + 1;
            exit when led /= previous;
        end loop;
        assert cycles = STEP_CYCLES
            report "step took " & integer'image(cycles) & " cycles" severity error;
        assert led = led_of(step mod VALUE_COUNT)
            report "value is not " & integer'image(step mod VALUE_COUNT) severity error;
    end loop;

    -- 途中で押しボタンを押すと、0 に戻る。
    pushbutton3 <= '0';
    next_sample;
    assert led = led_of(0)
        report "push button does not return to 0" severity error;
    pushbutton3 <= '1';

    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end sim;
