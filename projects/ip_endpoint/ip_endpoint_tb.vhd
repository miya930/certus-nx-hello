library ieee;
use     ieee.std_logic_1164.all;

entity ip_endpoint_tb is
end ip_endpoint_tb;

architecture sim of ip_endpoint_tb is

constant PHY_CLK_HZ     : positive := 125_000_000;
constant PHY_CLK_PERIOD : time := 1 sec / PHY_CLK_HZ;
constant SYS_CLK_HZ     : positive := 25_000_000;
constant SYS_CLK_PERIOD : time := 1 sec / SYS_CLK_HZ;

-- PHY の待ち時間は実機では 200 ms を超えるため、短くして確認する。
constant RESET_USEC     : positive := 1;
constant STARTUP_USEC   : positive := 2;

-- ブートローダが最初の 1 文字を送り始めるまで待つ上限。
-- 既定の 19200 baud では 1 文字に 521 マイクロ秒かかるため、それを大きく上回る値にする。
constant BOOT_TIMEOUT   : time := 5 ms;

-- MDIO は 1.6 Mbaud で 32 回の書き込みを行うため、その時間を上回る値にする。
constant MDIO_TIMEOUT   : time := 2 ms;

signal phy_clk      : std_logic := '0';
signal sys_clk      : std_logic := '0';
signal pushbutton3  : std_logic := '1';

-- 信号名は FTDI から見た向きで、TXD_UART はホストが送る線、RXD_UART はコアが送る線である。
signal txd_uart     : std_logic := '1';
signal rxd_uart     : std_logic;

signal rgmii_txclk  : std_logic;
signal rgmii_txctrl : std_logic;
signal rgmii_txd    : std_logic_vector(3 downto 0);
signal rgmii_rxclk  : std_logic := '0';
signal rgmii_rxctrl : std_logic := '0';
signal rgmii_rxd    : std_logic_vector(3 downto 0) := (others => '0');
signal mdio_clk     : std_logic;
signal mdio_data    : std_logic;
signal phy_rst_n    : std_logic;

signal led          : std_logic_vector(7 downto 0);
signal test_done    : boolean := false;

begin

uut : entity work.ip_endpoint
    generic map(
    RESET_USEC      => RESET_USEC,
    STARTUP_USEC    => STARTUP_USEC)
    port map(
    rgmii_txclk     => rgmii_txclk,
    rgmii_txctrl    => rgmii_txctrl,
    rgmii_txd       => rgmii_txd,
    rgmii_rxclk     => rgmii_rxclk,
    rgmii_rxctrl    => rgmii_rxctrl,
    rgmii_rxd       => rgmii_rxd,
    rgmii_mdio_clk  => mdio_clk,
    rgmii_mdio_data => mdio_data,
    rgmii_rst_n     => phy_rst_n,
    clk_customer2   => phy_clk,
    system_25m_clk  => sys_clk,
    pushbutton3     => pushbutton3,
    txd_uart        => txd_uart,
    rxd_uart        => rxd_uart,
    led             => led);

p_phy_clk : process
begin
    while not test_done loop
        phy_clk <= '0';
        wait for PHY_CLK_PERIOD / 2;
        phy_clk <= '1';
        wait for PHY_CLK_PERIOD / 2;
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
        wait until rising_edge(sys_clk);
    end loop;

    assert phy_rst_n = '0'
        report "reset: PHY is not held in reset" severity error;
    assert rxd_uart = '1'
        report "reset: UART is not idle" severity error;
    assert rgmii_txctrl = '0'
        report "reset: RGMII is transmitting" severity error;

    pushbutton3 <= '1';

    -- リセットの待ち時間が過ぎると、PHY のリセットが解除される。
    wait until phy_rst_n = '1' for BOOT_TIMEOUT;
    assert phy_rst_n = '1'
        report "PHY reset was not released" severity error;

    -- 起動の待ち時間の後、MDIO が設定を書き終える。LED0 の点灯がその合図になる。
    wait until led(0) = '0' for MDIO_TIMEOUT;
    assert led(0) = '0'
        report "MDIO did not finish writing the PHY registers" severity error;

    -- ブートローダが起動すると、UART が最初のスタートビットで 0 になる。
    -- CPU がスイッチコアと ConfigBus を抱えた構成でも動き出すことの確認になる。
    wait until rxd_uart = '0' for BOOT_TIMEOUT;
    assert rxd_uart = '0'
        report "bootloader did not send anything within the timeout" severity error;

    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end sim;
