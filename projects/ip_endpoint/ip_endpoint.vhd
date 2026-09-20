library ieee;
use     ieee.std_logic_1164.all;
use     work.cfgbus_common.all;
use     work.ptp_types.all;
use     work.switch_types.all;

library neorv32;

entity ip_endpoint is
    generic (
    IMEM_BYTES  : positive := 32*1024;
    DMEM_BYTES  : positive := 32*1024);
    port (
    rmii_txd        : out std_logic_vector(1 downto 0);
    rmii_tx_en      : out std_logic;
    rmii_rxd        : in  std_logic_vector(1 downto 0);
    rmii_crs_dv     : in  std_logic;
    rmii_ref_clk    : in  std_logic;
    system_25m_clk  : in  std_logic;
    pushbutton3     : in  std_logic;
    txd_uart        : in  std_logic;
    rxd_uart        : out std_logic;
    led             : out std_logic_vector(7 downto 0));
end ip_endpoint;

architecture rtl of ip_endpoint is

-- CPU、スイッチコア、ConfigBus を全て PHY からの 50 MHz の REF_CLK で動かし、クロックの乗り換えをなくす。
constant CORE_CLK_HZ    : positive := 50_000_000;

-- ConfigBus のデバイス番号。CPU から見たアドレスは、番号を 12 ビット左に寄せた位置に並ぶ。
constant DEV_SWITCH     : integer := 0;
constant DEV_MAILMAP    : integer := 1;

-- スイッチのポートは、PHY につながる RMII と、CPU につながる mailmap の 2 つである。
constant PORT_TOTAL     : positive := 2;
constant PORT_PHY       : natural := 0;
constant PORT_CPU       : natural := 1;

-- PHY 基板の LAN8742A が出す受信データは、REF_CLK の立ち上がりの TOINVLD 後から TOVAL 後まで変化する。
-- 入力遅延を d とすると、ホールドには TH - d <= TOINVLD、セットアップには TOVAL + TSU + d <= 周期が必要で、d をその中央に置く。
constant REF_CLK_PERIOD_NSEC    : real := 1.0e9 / real(CORE_CLK_HZ);
constant PHY_TOVAL_MAX_NSEC     : real := 15.0;
constant PHY_TOINVLD_MIN_NSEC   : real := 3.0;
constant FPGA_TSU_NSEC          : real := 0.0;
constant FPGA_TH_NSEC           : real := 3.32;
constant RXDAT_DELAY_NSEC       : real :=
    ((FPGA_TH_NSEC - PHY_TOINVLD_MIN_NSEC) + (REF_CLK_PERIOD_NSEC - PHY_TOVAL_MAX_NSEC - FPGA_TSU_NSEC)) / 2.0;

signal reset_p      : std_logic;

signal cfg_cmd      : cfgbus_cmd;
signal cfg_ack      : cfgbus_ack;
signal ack_switch   : cfgbus_ack;
signal ack_mailmap  : cfgbus_ack;

signal rx_data      : array_rx_m2s(PORT_TOTAL-1 downto 0);
signal tx_data      : array_tx_s2m(PORT_TOTAL-1 downto 0);
signal tx_ctrl      : array_tx_m2s(PORT_TOTAL-1 downto 0);

signal gpio         : std_ulogic_vector(31 downto 0);
signal xbus_adr     : std_ulogic_vector(31 downto 0);
signal xbus_wdat    : std_ulogic_vector(31 downto 0);
signal xbus_rdat    : std_logic_vector(31 downto 0);
signal xbus_cyc     : std_ulogic;
signal xbus_stb     : std_ulogic;
signal xbus_we      : std_ulogic;
signal xbus_ack     : std_logic;
signal xbus_err     : std_logic;

begin

-- 押しボタンは押している間だけ 0 になる。SatCat5 は正論理のリセットを取る。
reset_p <= not pushbutton3;

-- ConfigBus のコマンドは全デバイスに配り、応答はまとめて CPU へ返す。
cfg_ack <= cfgbus_merge(ack_switch, ack_mailmap);

-- CPU の外部バスは Wishbone なので、そのまま ConfigBus のホストにつながる。
-- アドレスの下位 18 ビットが、デバイス番号 8 ビットとレジスタ番号 10 ビットになる。
u_cfgbus : entity work.cfgbus_host_wishbone
    port map(
    cfg_cmd     => cfg_cmd,
    cfg_ack     => cfg_ack,
    interrupt   => open,
    wb_clk_i    => rmii_ref_clk,
    wb_rst_i    => reset_p,
    wb_adr_i    => std_logic_vector(xbus_adr(19 downto 2)),
    wb_cyc_i    => xbus_cyc,
    wb_dat_i    => std_logic_vector(xbus_wdat),
    wb_stb_i    => xbus_stb,
    wb_we_i     => xbus_we,
    wb_ack_o    => xbus_ack,
    wb_dat_o    => xbus_rdat,
    wb_err_o    => xbus_err);

-- 起動方法に内蔵ブートローダを選び、ファームウェアを UART から受け取る。
-- UART の信号名は FTDI から見た向きで、コアから見ると送受が入れ替わる。
u_cpu : entity neorv32.neorv32_top
    generic map(
    CLOCK_FREQUENCY  => CORE_CLK_HZ,
    BOOT_MODE_SELECT => 0,
    RISCV_ISA_C      => true,
    RISCV_ISA_M      => true,
    RISCV_ISA_Zicntr => true,
    IMEM_EN          => true,
    IMEM_SIZE        => IMEM_BYTES,
    DMEM_EN          => true,
    DMEM_SIZE        => DMEM_BYTES,
    XBUS_EN          => true,
    IO_GPIO_NUM      => led'length,
    IO_CLINT_EN      => true,
    IO_UART0_EN      => true)
    port map(
    clk_i       => rmii_ref_clk,
    rstn_i      => pushbutton3,
    gpio_o      => gpio,
    uart0_txd_o => rxd_uart,
    uart0_rxd_i => txd_uart,
    xbus_adr_o  => xbus_adr,
    xbus_dat_o  => xbus_wdat,
    xbus_we_o   => xbus_we,
    xbus_stb_o  => xbus_stb,
    xbus_cyc_o  => xbus_cyc,
    xbus_dat_i  => std_ulogic_vector(xbus_rdat),
    xbus_ack_i  => xbus_ack,
    xbus_err_i  => xbus_err);

-- CPU からはフレーム全体がメモリに見える。受信も送信も配列として読み書きする。
u_mailmap : entity work.port_mailmap
    generic map(
    DEV_ADDR    => DEV_MAILMAP)
    port map(
    rx_data     => rx_data(PORT_CPU),
    tx_data     => tx_data(PORT_CPU),
    tx_ctrl     => tx_ctrl(PORT_CPU),
    cfg_cmd     => cfg_cmd,
    cfg_ack     => ack_mailmap);

-- REF_CLK が止まったことを検出するため、別のクロックとして SYSTEM_25M_CLK を使う。
-- 送信データは、LAN8742A のセットアップ時間を満たすため REF_CLK の立ち上がりで出す。
u_phy : entity work.port_rmii
    generic map(
    MODE_CLKOUT => false,
    MODE_CLKDDR => false,
    RXDAT_DELAY => RXDAT_DELAY_NSEC)
    port map(
    rmii_txd    => rmii_txd,
    rmii_txen   => rmii_tx_en,
    rmii_txer   => open,
    rmii_rxd    => rmii_rxd,
    rmii_rxen   => rmii_crs_dv,
    rmii_rxer   => '0',
    rmii_clkin  => rmii_ref_clk,
    rmii_clkout => open,
    ref_time    => PORT_TIMEREF_NULL,
    rx_data     => rx_data(PORT_PHY),
    tx_data     => tx_data(PORT_PHY),
    tx_ctrl     => tx_ctrl(PORT_PHY),
    lock_refclk => system_25m_clk,
    reset_p     => reset_p);

u_core : entity work.switch_core
    generic map(
    DEV_ADDR        => DEV_SWITCH,
    CORE_CLK_HZ     => CORE_CLK_HZ,
    SUPPORT_PTP     => false,
    SUPPORT_VLAN    => false,
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
    cfg_ack         => ack_switch,
    log_txd         => open,
    scrub_req_t     => '0',
    core_clk        => rmii_ref_clk,
    core_reset_p    => reset_p);

-- LED は、出力を 0 にすると点灯する。
led <= not std_logic_vector(gpio(led'range));

end rtl;
