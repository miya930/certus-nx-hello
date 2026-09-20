library ieee;
use     ieee.std_logic_1164.all;

entity ip_endpoint_tb is
end ip_endpoint_tb;

architecture sim of ip_endpoint_tb is

constant REF_CLK_HZ     : positive := 50_000_000;
constant REF_CLK_PERIOD : time := 1 sec / REF_CLK_HZ;
constant SYS_CLK_HZ     : positive := 25_000_000;
constant SYS_CLK_PERIOD : time := 1 sec / SYS_CLK_HZ;

-- ブートローダが最初の 1 文字を送り始めるまで待つ上限。
-- 既定の 19200 baud では 1 文字に 521 マイクロ秒かかるため、それを大きく上回る値にする。
constant BOOT_TIMEOUT   : time := 5 ms;

signal ref_clk      : std_logic := '0';
signal sys_clk      : std_logic := '0';
signal pushbutton3  : std_logic := '1';

-- 信号名は FTDI から見た向きで、TXD_UART はホストが送る線、RXD_UART はコアが送る線である。
signal txd_uart     : std_logic := '1';
signal rxd_uart     : std_logic;

signal rmii_txd     : std_logic_vector(1 downto 0);
signal rmii_tx_en   : std_logic;
signal rmii_rxd     : std_logic_vector(1 downto 0) := "00";
signal rmii_crs_dv  : std_logic := '0';

signal led          : std_logic_vector(7 downto 0);
signal test_done    : boolean := false;

begin

uut : entity work.ip_endpoint
    port map(
    rmii_txd        => rmii_txd,
    rmii_tx_en      => rmii_tx_en,
    rmii_rxd        => rmii_rxd,
    rmii_crs_dv     => rmii_crs_dv,
    rmii_ref_clk    => ref_clk,
    system_25m_clk  => sys_clk,
    pushbutton3     => pushbutton3,
    txd_uart        => txd_uart,
    rxd_uart        => rxd_uart,
    led             => led);

p_ref_clk : process
begin
    while not test_done loop
        ref_clk <= '0';
        wait for REF_CLK_PERIOD / 2;
        ref_clk <= '1';
        wait for REF_CLK_PERIOD / 2;
    end loop;
    wait;
end process;

p_sys_clk : process
begin
    while not test_done loop
        sys_clk <= '0';
        wait for SYS_CLK_PERIOD / 2;
        sys_clk <= '1';
        wait for SYS_CLK_PERIOD / 2;
    end loop;
    wait;
end process;

p_test : process
begin
    -- 押しボタンを押した状態から始める。
    pushbutton3 <= '0';
    for n in 1 to 10 loop
        wait until rising_edge(ref_clk);
    end loop;

    assert led = (led'range => '1')
        report "reset: an LED is lit" severity error;
    assert rxd_uart = '1'
        report "reset: UART is not idle" severity error;
    assert rmii_tx_en = '0'
        report "reset: RMII is transmitting" severity error;

    pushbutton3 <= '1';

    -- ブートローダが起動すると、UART が最初のスタートビットで 0 になる。
    -- CPU が ConfigBus とスイッチコアを抱えた構成でも動き出すことの確認になる。
    wait until rxd_uart = '0' for BOOT_TIMEOUT;
    assert rxd_uart = '0'
        report "bootloader did not send anything within the timeout" severity error;

    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end sim;
