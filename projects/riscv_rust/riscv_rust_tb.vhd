library ieee;
use     ieee.std_logic_1164.all;

entity riscv_rust_tb is
end riscv_rust_tb;

architecture sim of riscv_rust_tb is

constant CLK_HZ     : positive := 25_000_000;
constant CLK_PERIOD : time := 1 sec / CLK_HZ;

-- ブートローダが最初の 1 文字を送り始めるまで待つ上限。
-- 既定の 19200 baud では 1 文字に 521 マイクロ秒かかるため、それを大きく上回る値にする。
constant BOOT_TIMEOUT : time := 5 ms;

signal clk          : std_logic := '0';
signal pushbutton3  : std_logic := '1';
signal rxd_uart     : std_logic := '1';
signal txd_uart     : std_logic;
signal led          : std_logic_vector(7 downto 0);
signal test_done    : boolean := false;

begin

uut : entity work.riscv_rust
    generic map(
    CLK_HZ => CLK_HZ)
    port map(
    system_25m_clk  => clk,
    pushbutton3     => pushbutton3,
    rxd_uart        => rxd_uart,
    txd_uart        => txd_uart,
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
begin
    -- 押しボタンを押した状態から始める。
    pushbutton3 <= '0';
    for n in 1 to 10 loop
        wait until rising_edge(clk);
    end loop;

    assert led = (led'range => '1')
        report "reset: an LED is lit" severity error;
    assert txd_uart = '1'
        report "reset: UART is not idle" severity error;

    pushbutton3 <= '1';

    -- ブートローダが起動すると、UART が最初のスタートビットで 0 になる。
    wait until txd_uart = '0' for BOOT_TIMEOUT;
    assert txd_uart = '0'
        report "bootloader did not send anything within the timeout" severity error;

    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end sim;
