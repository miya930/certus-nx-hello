library ieee;
use     ieee.std_logic_1164.all;

entity led_sequence_tb is
end led_sequence_tb;

architecture sim of led_sequence_tb is

-- 点灯を切り替える間隔が 10 サイクルになるようにして、短い時間で確認する。
constant CLK_HZ         : positive := 10_000;
constant STEP_MSEC      : positive := 1;
constant STEP_CYCLES    : positive := CLK_HZ / 1000 * STEP_MSEC;
constant CLK_PERIOD     : time := 1 sec / CLK_HZ;
constant LED_COUNT      : positive := 8;

-- LED は 0 で点灯するため、点灯する 1 つだけを 0 にする。
function one_lit(index : natural) return std_logic_vector is
    variable pattern : std_logic_vector(LED_COUNT-1 downto 0) := (others => '1');
begin
    pattern(index) := '0';
    return pattern;
end function;

-- 点灯している LED の数を数え、常に 1 つだけであることを確かめる。
function lit_count(pattern : std_logic_vector) return natural is
    variable total : natural := 0;
begin
    for n in pattern'range loop
        if pattern(n) = '0' then
            total := total + 1;
        end if;
    end loop;
    return total;
end function;

signal clk          : std_logic := '0';
signal pushbutton3  : std_logic := '1';
signal led          : std_logic_vector(LED_COUNT-1 downto 0);
signal test_done    : boolean := false;

begin

uut : entity work.led_sequence
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
    variable previous   : std_logic_vector(LED_COUNT-1 downto 0);
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
    assert led = one_lit(0)
        report "reset: LED0 is not lit" severity error;
    pushbutton3 <= '1';

    -- 2 周ぶん、点灯する LED が STEP_CYCLES サイクルごとに 1 つずつ移ることを確認する。
    for step in 1 to 2*LED_COUNT loop
        previous := led;
        cycles := 0;
        loop
            next_sample;
            cycles := cycles + 1;
            assert lit_count(led) = 1
                report "number of lit LEDs is " & integer'image(lit_count(led)) severity error;
            exit when led /= previous;
        end loop;
        assert cycles = STEP_CYCLES
            report "step took " & integer'image(cycles) & " cycles" severity error;
        assert led = one_lit(step mod LED_COUNT)
            report "lit LED is not the next one" severity error;
    end loop;

    -- 途中で押しボタンを押すと、LED0 に戻る。
    pushbutton3 <= '0';
    next_sample;
    assert led = one_lit(0)
        report "push button does not return to LED0" severity error;
    pushbutton3 <= '1';

    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end sim;
