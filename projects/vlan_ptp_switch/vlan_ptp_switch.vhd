library ieee;
use     ieee.std_logic_1164.all;
use     work.cfgbus_common.all;
use     work.common_primitives.all;
use     work.ptp_types.all;
use     work.switch_types.all;

entity vlan_ptp_switch is
    generic (
    PORT_COUNT  : positive := 4);
    port (
    rmii_txd        : out std_logic_vector(2*PORT_COUNT-1 downto 0);
    rmii_tx_en      : out std_logic_vector(PORT_COUNT-1 downto 0);
    rmii_rxd        : in  std_logic_vector(2*PORT_COUNT-1 downto 0);
    rmii_crs_dv     : in  std_logic_vector(PORT_COUNT-1 downto 0);
    rmii_ref_clk    : in  std_logic;
    system_25m_clk  : in  std_logic;
    reset_p         : in  std_logic);
end vlan_ptp_switch;

architecture rtl of vlan_ptp_switch is

-- 全ポートが PHY からの 50 MHz の REF_CLK を共有し、スイッチコアも同じクロックで動かす。
-- 1 バイト幅のパイプラインなので、全ポート合計で 400 Mbps まで処理できる。
constant CORE_CLK_HZ    : positive := 50_000_000;
constant SYSTEM_CLK_HZ  : positive := 25_000_000;

-- PTP のタイムスタンプは、PLL を使わない coarse 方式で RMII のクロックへ乗せ換える。
-- coarse 方式は RMII のクロックが VCLKA の 2 倍より速いことを求めるため、SYSTEM_25M_CLK を 2 分周して VCLKA にする。
constant VCLKA_HZ       : positive := SYSTEM_CLK_HZ / 2;
constant VCONFIG        : vernier_config := create_vernier_coarse(VCLKA_HZ);

-- ConfigBus は、スイッチの内部ポートにつないだ port_cfgbus が Ethernet フレームで受け付ける。
-- 内部ポートは RMII ポートの後ろの番号に置く。
constant PORT_TOTAL     : positive := PORT_COUNT + 1;
constant CFG_PORT       : natural := PORT_COUNT;
constant CFG_DEV_ADDR   : integer := 0;

-- PHY 基板の LAN8742A が出す受信データは、REF_CLK の立ち上がりの TOINVLD 後から TOVAL 後まで変化する。
-- 入力遅延を d とすると、ホールドには TH - d <= TOINVLD、セットアップには TOVAL + TSU + d <= 周期が必要で、d をその中央に置く。
-- LAN8742A の値は DS_LAN8742_00001989A.md の RMII Timing (REF_CLK In Mode) の表、
-- Certus-NX の値は FPGA-DS-02078-2-5-Certus-NX-Family.md の External Switching Characteristics の表の -8 で、PLL を使わない場合による。
-- この表は専用のクロック入力ピンでの値で、PMOD の REF_CLK は一般のピンなので、実機での確認が必要である。
constant REF_CLK_PERIOD_NSEC    : real := 1.0e9 / real(CORE_CLK_HZ);
constant PHY_TOVAL_MAX_NSEC     : real := 15.0;
constant PHY_TOINVLD_MIN_NSEC   : real := 3.0;
constant FPGA_TSU_NSEC          : real := 0.0;
constant FPGA_TH_NSEC           : real := 3.32;
constant RXDAT_DELAY_NSEC       : real :=
    ((FPGA_TH_NSEC - PHY_TOINVLD_MIN_NSEC) + (REF_CLK_PERIOD_NSEC - PHY_TOVAL_MAX_NSEC - FPGA_TSU_NSEC)) / 2.0;

signal vclka    : std_logic := '0';
signal ref_time : port_timeref;

signal cfg_cmd  : cfgbus_cmd;
signal cfg_ack  : cfgbus_ack;

signal rx_data  : array_rx_m2s(PORT_TOTAL-1 downto 0);
signal tx_data  : array_tx_s2m(PORT_TOTAL-1 downto 0);
signal tx_ctrl  : array_tx_m2s(PORT_TOTAL-1 downto 0);

begin

p_vclka : process(system_25m_clk)
begin
    if rising_edge(system_25m_clk) then
        vclka <= not vclka;
    end if;
end process;

u_ptp_ref : entity work.ptp_counter_gen
    generic map(
    VCONFIG     => VCONFIG)
    port map(
    vclka       => vclka,
    vclkb       => '0',
    vreset_p    => reset_p,
    ref_time    => ref_time);

u_cfgbus : entity work.port_cfgbus
    port map(
    cfg_cmd     => cfg_cmd,
    cfg_ack     => cfg_ack,
    rx_data     => rx_data(CFG_PORT),
    tx_data     => tx_data(CFG_PORT),
    tx_ctrl     => tx_ctrl(CFG_PORT),
    sys_clk     => rmii_ref_clk,
    reset_p     => reset_p);

gen_port : for n in 0 to PORT_COUNT-1 generate
    -- REF_CLK が止まったことを検出するため、別のクロックとして SYSTEM_25M_CLK を使う。
    -- 送信データは、LAN8742A のセットアップ時間を満たすため REF_CLK の立ち上がりで出す。
    u_port : entity work.port_rmii
        generic map(
        MODE_CLKOUT => false,
        MODE_CLKDDR => false,
        RXDAT_DELAY => RXDAT_DELAY_NSEC,
        VCONFIG     => VCONFIG)
        port map(
        rmii_txd    => rmii_txd(2*n+1 downto 2*n),
        rmii_txen   => rmii_tx_en(n),
        rmii_txer   => open,
        rmii_rxd    => rmii_rxd(2*n+1 downto 2*n),
        rmii_rxen   => rmii_crs_dv(n),
        rmii_rxer   => '0',
        rmii_clkin  => rmii_ref_clk,
        rmii_clkout => open,
        ref_time    => ref_time,
        rx_data     => rx_data(n),
        tx_data     => tx_data(n),
        tx_ctrl     => tx_ctrl(n),
        lock_refclk => system_25m_clk,
        reset_p     => reset_p);
end generate;

u_core : entity work.switch_core
    generic map(
    DEV_ADDR        => CFG_DEV_ADDR,
    CORE_CLK_HZ     => CORE_CLK_HZ,
    SUPPORT_PTP     => true,
    SUPPORT_VLAN    => true,
    PORT_COUNT      => PORT_TOTAL,
    DATAPATH_BYTES  => 1,
    OBUF_KBYTES     => 8)
    port map(
    ports_rx_data   => rx_data,
    ports_tx_data   => tx_data,
    ports_tx_ctrl   => tx_ctrl,
    err_ports       => open,
    err_switch      => open,
    errvec_t        => open,
    cfg_cmd         => cfg_cmd,
    cfg_ack         => cfg_ack,
    log_txd         => open,
    scrub_req_t     => '0',
    core_clk        => rmii_ref_clk,
    core_reset_p    => reset_p);

end rtl;
