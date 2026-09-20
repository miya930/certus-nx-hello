library ieee;
use     ieee.numeric_std.all;
use     ieee.std_logic_1164.all;
use     work.common_functions.all;
use     work.config_mdio_rom_creation.all;
use     work.eth_frame_common.all;
use     work.switch_types.all;

entity rgmii_phy_test is
    generic (
    PHY_ADDR        : natural := 0;
    RESET_USEC      : positive := 1_000;
    STARTUP_USEC    : positive := 200_000);
    port (
    rgmii_txclk     : out std_logic;
    rgmii_txctrl    : out std_logic;
    rgmii_txd       : out std_logic_vector(3 downto 0);
    rgmii_rxclk     : in  std_logic;
    rgmii_rxctrl    : in  std_logic;
    rgmii_rxd       : in  std_logic_vector(3 downto 0);
    rgmii_mdio_clk  : out std_logic;
    rgmii_mdio_data : out std_logic;
    rgmii_rst_n     : out std_logic;
    clk_customer2   : in  std_logic;
    pushbutton3     : in  std_logic;
    led             : out std_logic_vector(1 downto 0));
end rgmii_phy_test;

architecture rtl of rgmii_phy_test is

-- RGMII の 1000 Mbps は、ボードの 125 MHz の発振器からそのまま作る。
constant CLK_HZ         : positive := 125_000_000;
constant MDIO_BAUD      : positive := 1_600_000;

-- PHY のリセットは 1 us 以上、リセット解除から MDIO の操作までは 195 us 以上あける。
-- 電源投入からは 200 ms 以上あけるため、既定値は長いほうに合わせる。
-- テストベンチでは、この 2 つを短くして確認する。
constant RESET_CYCLES   : positive := CLK_HZ / 1_000_000 * RESET_USEC;
constant STARTUP_CYCLES : positive := CLK_HZ / 1_000_000 * STARTUP_USEC;

-- 送信するテストフレームは、宛先をブロードキャスト、送信元をローカル管理アドレスにする。
-- EtherType は IEEE が試験用に割り当てた値を使う。
constant TEST_DST       : mac_addr_t := x"FFFFFFFFFFFF";
constant TEST_SRC       : mac_addr_t := x"5A5A00000001";
constant TEST_ETYPE     : mac_type_t := x"88B5";
constant TEST_NBYTES    : positive := 256;

-- MDIO で書き込むレジスタは、拡張レジスタ空間にある。
-- 拡張レジスタは、REGCR にアドレス指定を書き、ADDAR にレジスタ番号を書き、
-- REGCR にデータ指定を書き、ADDAR に値を書く、という 4 回の書き込みで操作する。
constant REG_REGCR      : natural := 16#0D#;
constant REG_ADDAR      : natural := 16#0E#;
constant REGCR_ADDRESS  : natural := 16#001F#;
constant REGCR_DATA     : natural := 16#401F#;

-- RGMIICTL は、RGMII を有効にし、送信と受信のクロックをデータに対してずらす設定にする。
-- FIFO のしきい値は初期値のままにする。
constant REG_RGMIICTL   : natural := 16#0032#;
constant RGMIICTL_VALUE : natural := 16#00D3#;

-- RGMIIDCTL は、送信と受信のクロックのずれを 2.00 ns にする。
-- FPGA 側ではクロックをずらさないため、ずれは PHY だけで作る。
constant REG_RGMIIDCTL  : natural := 16#0086#;
constant RGMIIDCTL_VALUE : natural := 16#0077#;

-- 最初の書き込みまでの待ちは、起動の待ちで足りているため 0 ms にする。
function mdio_write(reg, value : natural) return std_logic_vector is
begin
    return config_mdio_rom_cmd(0, PHY_ADDR, REG_REGCR, REGCR_ADDRESS)
         & config_mdio_rom_cmd(0, PHY_ADDR, REG_ADDAR, reg)
         & config_mdio_rom_cmd(0, PHY_ADDR, REG_REGCR, REGCR_DATA)
         & config_mdio_rom_cmd(0, PHY_ADDR, REG_ADDAR, value);
end function;

-- 拡張レジスタ 1 つにつき 4 回の書き込みが必要になる。
constant CMD_COUNT  : positive := 8;
constant ROM_VECTOR : std_logic_vector(32*CMD_COUNT-1 downto 0) :=
    mdio_write(REG_RGMIICTL, RGMIICTL_VALUE) &
    mdio_write(REG_RGMIIDCTL, RGMIIDCTL_VALUE);

signal reset_p      : std_logic;
signal phy_reset_n  : std_logic := '0';
signal startup_p    : std_logic := '1';
signal count        : natural range 0 to STARTUP_CYCLES-1 := 0;

signal mdio_oe      : std_logic;
signal status_done  : std_logic;

signal tx_data      : port_tx_s2m;
signal tx_ctrl      : port_tx_m2s;
signal rx_data      : port_rx_m2s;
signal frame_data   : byte_t;
signal frame_last   : std_logic;
signal frame_valid  : std_logic;
signal frame_ready  : std_logic;
signal frame_toggle : std_logic;
signal prev_toggle  : std_logic := '0';
signal frame_count  : unsigned(17 downto 0) := (others => '0');

begin

-- 押しボタンは、押している間だけ 0 になる。
reset_p <= not pushbutton3;

-- 電源投入とリセットの後、PHY のリセットを解除してから起動の待ち時間を数える。
p_startup : process(clk_customer2)
begin
    if rising_edge(clk_customer2) then
        if reset_p = '1' then
            count <= 0;
            phy_reset_n <= '0';
            startup_p <= '1';
        elsif count = STARTUP_CYCLES-1 then
            startup_p <= '0';
        else
            count <= count + 1;
            if count = RESET_CYCLES then
                phy_reset_n <= '1';
            end if;
        end if;
    end if;
end process;

rgmii_rst_n <= phy_reset_n;

u_mdio : entity work.config_mdio_rom
    generic map(
    CLKREF_HZ   => CLK_HZ,
    MDIO_BAUD   => MDIO_BAUD,
    ROM_VECTOR  => ROM_VECTOR)
    port map(
    mdio_clk    => rgmii_mdio_clk,
    mdio_data   => rgmii_mdio_data,
    mdio_oe     => mdio_oe,
    status_done => status_done,
    ref_clk     => clk_customer2,
    reset_p     => startup_p);

-- レジスタを書くだけで読まないため、MDIO のデータ線は常に FPGA が駆動する。
-- PHY がこの線を駆動するのは読み出しの応答のときだけで、この回路では起きない。

-- 送信元は、ポートが送信できる状態になってから動かす。
-- ポートがリセット中でも tx_ctrl.ready は 1 になり、送ったデータが捨てられるためである。
u_traffic : entity work.eth_traffic_src
    generic map(
    HDR_DST     => TEST_DST,
    HDR_SRC     => TEST_SRC,
    HDR_ETYPE   => TEST_ETYPE,
    FRM_NBYTES  => TEST_NBYTES)
    port map(
    out_data    => frame_data,
    out_last    => frame_last,
    out_valid   => frame_valid,
    out_ready   => frame_ready,
    out_pkt_t   => frame_toggle,
    clk         => clk_customer2,
    reset_p     => tx_ctrl.reset_p);

tx_data.data    <= frame_data;
tx_data.last    <= frame_last;
tx_data.valid   <= frame_valid;
frame_ready     <= tx_ctrl.ready;

-- 送信と受信のクロックのずれは PHY が作るため、FPGA 側では遅延を入れない。
u_rgmii : entity work.port_rgmii
    generic map(
    RXCLK_DELAY => -1.0,
    RXDAT_DELAY => -1.0)
    port map(
    rgmii_txc   => rgmii_txclk,
    rgmii_txd   => rgmii_txd,
    rgmii_txctl => rgmii_txctrl,
    rgmii_rxc   => rgmii_rxclk,
    rgmii_rxd   => rgmii_rxd,
    rgmii_rxctl => rgmii_rxctrl,
    rx_data     => rx_data,
    tx_data     => tx_data,
    tx_ctrl     => tx_ctrl,
    clk_125     => clk_customer2,
    clk_txc     => clk_customer2,
    reset_p     => startup_p);

-- 送信したフレームを数え、目で見える速さで LED を点滅させる。
p_count : process(clk_customer2)
begin
    if rising_edge(clk_customer2) then
        prev_toggle <= frame_toggle;
        if prev_toggle /= frame_toggle then
            frame_count <= frame_count + 1;
        end if;
    end if;
end process;

-- LED は 0 で点灯する。
led(0) <= not status_done;
led(1) <= not frame_count(frame_count'left);

end rtl;
