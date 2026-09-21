library ieee;
use     ieee.std_logic_1164.all;
use     work.cfgbus_common.all;
use     work.config_mdio_rom_creation.all;
use     work.switch_types.all;

library neorv32;

entity managed_switch is
    generic (
    IMEM_BYTES      : positive := 32*1024;
    DMEM_BYTES      : positive := 32*1024;
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
    rmii_ref_clk    : in  std_logic_vector(1 downto 0);
    rmii_txd        : out std_logic_vector(3 downto 0);
    rmii_txen       : out std_logic_vector(1 downto 0);
    rmii_rxd        : in  std_logic_vector(3 downto 0);
    rmii_crs_dv     : in  std_logic_vector(1 downto 0);
    clk_customer2   : in  std_logic;
    system_25m_clk  : in  std_logic;
    pushbutton3     : in  std_logic;
    txd_uart        : in  std_logic;
    rxd_uart        : out std_logic;
    led             : out std_logic_vector(7 downto 0));
end managed_switch;

architecture rtl of managed_switch is

-- RGMII の 1000 Mbps は、ボードの 125 MHz の発振器からそのまま作る。
constant PHY_CLK_HZ     : positive := 125_000_000;
constant MDIO_BAUD      : positive := 1_600_000;

-- スイッチコア、ConfigBus、CPU は 25 MHz で動かす。
-- 配置配線の結果が 125 MHz に届かないためで、1 バイト幅のパイプラインで 200 Mbps を扱える。
-- SatCat5 のポートは自前のクロックを持ち、スイッチコアへの受け渡しでクロックを乗り換える。
constant CORE_CLK_HZ    : positive := 25_000_000;

-- PHY のリセットは 1 us 以上、リセット解除から MDIO の操作までは 195 us 以上あける。
-- 電源投入からは 200 ms 以上あけるため、既定値は長いほうに合わせる。
-- テストベンチでは、この 2 つを短くして確認する。
constant RESET_CYCLES   : positive := PHY_CLK_HZ / 1_000_000 * RESET_USEC;
constant STARTUP_CYCLES : positive := PHY_CLK_HZ / 1_000_000 * STARTUP_USEC;

-- MDIO で書き込むレジスタは、拡張レジスタ空間にある。
-- 拡張レジスタは、REGCR にアドレス指定を書き、ADDAR にレジスタ番号を書き、
-- REGCR にデータ指定を書き、ADDAR に値を書く、という 4 回の書き込みで操作する。
constant REG_REGCR      : natural := 16#0D#;
constant REG_ADDAR      : natural := 16#0E#;
constant REGCR_ADDRESS  : natural := 16#001F#;
constant REGCR_DATA     : natural := 16#401F#;

-- RGMIICTL は、RGMII を有効にし、送信と受信のクロックをデータに対してずらす設定にする。
constant REG_RGMIICTL   : natural := 16#0032#;
constant RGMIICTL_VALUE : natural := 16#00D3#;

-- RGMIIDCTL は、送信と受信のクロックのずれを 2.00 ns にする。
-- FPGA 側ではクロックをずらさないため、ずれは PHY だけで作る。
constant REG_RGMIIDCTL   : natural := 16#0086#;
constant RGMIIDCTL_VALUE : natural := 16#0077#;

-- LAN8720 のモジュールはそれぞれ 50 MHz の発振器を持ち、LAN8720 と FPGA が同じ REF_CLK を受ける。
-- LAN8720 の受信データは、REF_CLK の立ち上がりの TOHOLD 後から TOVAL 後まで変化する。
-- FPGA の入力レジスタのホールド時間 TH は TOHOLD より長いため、入力遅延 d を入れて立ち上がりで取り込む。
-- ホールドには TH - d <= TOHOLD、セットアップには TOVAL + TSU + d <= 周期が必要で、d をその中央に置く。
-- LAN8720 の値は REF_CLK In Mode のもので、Certus-NX の値は -8 で専用のクロック入力を PLL なしで使う場合のものである。
-- PMOD の REF_CLK は一般のピンから入るため、実機での確認が必要である。
constant RMII_CLK_HZ            : positive := 50_000_000;
constant RMII_PERIOD_NSEC       : real := 1.0e9 / real(RMII_CLK_HZ);
constant LAN8720_TOVAL_NSEC     : real := 14.0;
constant LAN8720_TOHOLD_NSEC    : real := 3.0;
constant FPGA_TSU_NSEC          : real := 0.0;
constant FPGA_TH_NSEC           : real := 3.32;
constant RMII_RXDAT_DELAY_NSEC  : real :=
    ((FPGA_TH_NSEC - LAN8720_TOHOLD_NSEC) + (RMII_PERIOD_NSEC - LAN8720_TOVAL_NSEC - FPGA_TSU_NSEC)) / 2.0;

-- ConfigBus のデバイス番号。CPU から見たアドレスは、番号を 12 ビット左に寄せた位置に並ぶ。
constant DEV_SWITCH     : integer := 0;
constant DEV_MAILMAP    : integer := 1;

-- スイッチのポートは、DP83867 につながる RGMII、LAN8720 につながる RMII、CPU につながる mailmap の順に並べる。
-- RMII のポートの数は、PMOD に出した REF_CLK の数で決まる。
constant RMII_COUNT     : positive := rmii_ref_clk'length;
constant PORT_RGMII     : natural := 0;
constant PORT_RMII      : natural := 1;
constant PORT_CPU       : natural := PORT_RMII + RMII_COUNT;
constant PORT_TOTAL     : positive := PORT_CPU + 1;

-- 最初の書き込みまでの待ちは、起動の待ちで足りているため 0 ms にする。
function mdio_write(reg, value : natural) return std_logic_vector is
begin
    return config_mdio_rom_cmd(0, PHY_ADDR, REG_REGCR, REGCR_ADDRESS)
         & config_mdio_rom_cmd(0, PHY_ADDR, REG_ADDAR, reg)
         & config_mdio_rom_cmd(0, PHY_ADDR, REG_REGCR, REGCR_DATA)
         & config_mdio_rom_cmd(0, PHY_ADDR, REG_ADDAR, value);
end function;

-- スイッチコアは 25 MHz で 200 Mbps までしか扱えないため、1000BASE-T を広告させない。
-- CFG1 のビット 9 と 8 が 1000BASE-T の全二重と半二重の広告で、両方を落とす。
-- 広告を変えた後は、BMCR で自動交渉をやり直させる。
constant REG_CFG1       : natural := 16#09#;
constant CFG1_VALUE     : natural := 16#0000#;
constant REG_BMCR       : natural := 16#00#;
constant BMCR_VALUE     : natural := 16#1200#;

-- 拡張レジスタ 1 つにつき 4 回、通常のレジスタは 1 回の書き込みになる。
constant CMD_COUNT  : positive := 10;
constant ROM_VECTOR : std_logic_vector(32*CMD_COUNT-1 downto 0) :=
    mdio_write(REG_RGMIICTL, RGMIICTL_VALUE) &
    mdio_write(REG_RGMIIDCTL, RGMIIDCTL_VALUE) &
    config_mdio_rom_cmd(0, PHY_ADDR, REG_CFG1, CFG1_VALUE) &
    config_mdio_rom_cmd(0, PHY_ADDR, REG_BMCR, BMCR_VALUE);

signal reset_p      : std_logic;
signal phy_reset_n  : std_logic := '0';
signal startup_p    : std_logic := '1';
signal count        : natural range 0 to STARTUP_CYCLES-1 := 0;
signal mdio_done    : std_logic;

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
signal wb_ack       : std_logic;
signal wb_err       : std_logic;
signal wb_rdat      : std_logic_vector(31 downto 0);
signal xbus_ack     : std_logic := '0';
signal xbus_err     : std_logic := '0';

begin

-- 押しボタンは押している間だけ 0 になる。SatCat5 は正論理のリセットを取る。
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

-- レジスタを書くだけで読まないため、MDIO のデータ線は常に FPGA が駆動する。
u_mdio : entity work.config_mdio_rom
    generic map(
    CLKREF_HZ   => PHY_CLK_HZ,
    MDIO_BAUD   => MDIO_BAUD,
    ROM_VECTOR  => ROM_VECTOR)
    port map(
    mdio_clk    => rgmii_mdio_clk,
    mdio_data   => rgmii_mdio_data,
    mdio_oe     => open,
    status_done => mdio_done,
    ref_clk     => clk_customer2,
    reset_p     => startup_p);

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
    rx_data     => rx_data(PORT_RGMII),
    tx_data     => tx_data(PORT_RGMII),
    tx_ctrl     => tx_ctrl(PORT_RGMII),
    clk_125     => clk_customer2,
    clk_txc     => clk_customer2,
    reset_p     => startup_p);

-- 送信データは REF_CLK の立ち上がりで出す。
-- FPGA の出力の遅れを足しても、次の立ち上がりまでに LAN8720 のセットアップ時間が残る。
-- RXER は使わず、受信の誤りはスイッチコアが FCS で見つける。
-- REF_CLK が止まったことを検出するため、別のクロックとして SYSTEM_25M_CLK を使う。
gen_rmii : for n in 0 to RMII_COUNT-1 generate
    u_rmii : entity work.port_rmii
        generic map(
        MODE_CLKOUT => false,
        MODE_CLKDDR => false,
        RXDAT_DELAY => RMII_RXDAT_DELAY_NSEC)
        port map(
        rmii_txd    => rmii_txd(2*n+1 downto 2*n),
        rmii_txen   => rmii_txen(n),
        rmii_txer   => open,
        rmii_rxd    => rmii_rxd(2*n+1 downto 2*n),
        rmii_rxen   => rmii_crs_dv(n),
        rmii_rxer   => '0',
        rmii_clkin  => rmii_ref_clk(n),
        rmii_clkout => open,
        rx_data     => rx_data(PORT_RMII + n),
        tx_data     => tx_data(PORT_RMII + n),
        tx_ctrl     => tx_ctrl(PORT_RMII + n),
        lock_refclk => system_25m_clk,
        reset_p     => reset_p);
end generate;

-- ConfigBus のコマンドは全デバイスに配り、応答はまとめて CPU へ返す。
cfg_ack <= cfgbus_merge(ack_switch, ack_mailmap);

-- CPU の外部バスは Wishbone なので、そのまま ConfigBus のホストにつながる。
-- アドレスの下位 18 ビットが、デバイス番号 8 ビットとレジスタ番号 10 ビットになる。
u_cfgbus : entity work.cfgbus_host_wishbone
    port map(
    cfg_cmd     => cfg_cmd,
    cfg_ack     => cfg_ack,
    interrupt   => open,
    wb_clk_i    => system_25m_clk,
    wb_rst_i    => reset_p,
    wb_adr_i    => std_logic_vector(xbus_adr(19 downto 2)),
    wb_cyc_i    => xbus_cyc,
    wb_dat_i    => std_logic_vector(xbus_wdat),
    wb_stb_i    => xbus_stb,
    wb_we_i     => xbus_we,
    wb_ack_o    => wb_ack,
    wb_dat_o    => wb_rdat,
    wb_err_o    => wb_err);

-- NEORV32 の外部バスは、ストローブを 1 サイクルしか出さず、応答をその次のサイクルから受け付ける。
-- ブリッジは書き込みの応答をストローブと同じサイクルに返すため、1 サイクル遅らせて合わせる。
-- 読み出しのデータは応答と同時に確定するため、同じタイミングで取り込む。
p_xbus_rsp : process(system_25m_clk)
begin
    if rising_edge(system_25m_clk) then
        xbus_ack  <= wb_ack;
        xbus_err  <= wb_err;
        xbus_rdat <= wb_rdat;
    end if;
end process;

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
    clk_i       => system_25m_clk,
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

u_core : entity work.switch_core
    generic map(
    DEV_ADDR        => DEV_SWITCH,
    CORE_CLK_HZ     => CORE_CLK_HZ,
    SUPPORT_PTP     => false,
    SUPPORT_VLAN    => false,
    PORT_COUNT      => PORT_TOTAL,
    DATAPATH_BYTES  => 1,
    OBUF_KBYTES     => 2)
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
    core_clk        => system_25m_clk,
    core_reset_p    => reset_p);

-- LED は、出力を 0 にすると点灯する。
-- 最下位は MDIO の設定が終わったことを示し、残りは CPU が動かす。
led(0) <= not mdio_done;
led(led'left downto 1) <= not std_logic_vector(gpio(led'left downto 1));

end rtl;
