library ieee;
use     ieee.std_logic_1164.all;
use     ieee.numeric_std.all;
use     work.eth_frame_common.all;

entity managed_switch_tb is
end managed_switch_tb;

architecture sim of managed_switch_tb is

constant PHY_CLK_HZ     : positive := 125_000_000;
constant PHY_CLK_PERIOD : time := 1 sec / PHY_CLK_HZ;
constant SYS_CLK_HZ     : positive := 25_000_000;
constant SYS_CLK_PERIOD : time := 1 sec / SYS_CLK_HZ;
constant RMII_CLK_HZ    : positive := 50_000_000;
constant RMII_CLK_PERIOD : time := 1 sec / RMII_CLK_HZ;

-- LAN8720 のモジュールはそれぞれ発振器を持つため、REF_CLK の位相をポートごとにずらす。
constant RMII_COUNT     : positive := 2;
constant RMII_PHASE_STEP : time := 7 ns;

-- 基板では DP83867 が 100BASE-TX でリンクするため、RGMII も 100 Mbps の 25 MHz で動かす。
constant RGMII_CLK_HZ   : positive := 25_000_000;
constant RGMII_CLK_PERIOD : time := 1 sec / RGMII_CLK_HZ;

-- DP83867 は、送信と受信のクロックをデータに対して 2 ns ずらす。
constant RGMII_SKEW     : time := 2 ns;

-- RGMII の受信線は、フレームの間にリンクの状態を載せる。
-- 最下位からリンクあり、速度 100 Mbps、全二重を表す。
constant RGMII_INBAND   : std_logic_vector(3 downto 0) := "1011";

-- LAN8720 は受信データを REF_CLK の立ち上がりの 3 ns 後から 14 ns 後までに変えるため、その間で変える。
constant RMII_RX_DELAY  : time := 8 ns;

-- PHY の待ち時間は実機では 200 ms を超えるため、短くして確認する。
constant RESET_USEC     : positive := 1;
constant STARTUP_USEC   : positive := 2;

-- ブートローダが最初の 1 文字を送り始めるまで待つ上限。
-- 既定の 19200 baud では 1 文字に 521 マイクロ秒かかるため、それを大きく上回る値にする。
constant BOOT_TIMEOUT   : time := 5 ms;

-- MDIO は 1.6 Mbaud で 32 回の書き込みを行うため、その時間を上回る値にする。
constant MDIO_TIMEOUT   : time := 2 ms;

-- 100 Mbps で 72 バイトを送る 5.8 マイクロ秒を大きく上回る値にする。
constant FRAME_TIMEOUT  : time := 100 us;

-- 最短のフレームを送る。長さは FCS を含まない。
constant FRAME_BYTES    : positive := 60;
constant MAX_BYTES      : positive := 128;
constant ETHERTYPE      : mac_type_t := x"88B5";
constant PREAMBLE       : byte_array_t(0 to 7) := (
    ETH_AMBLE_PRE, ETH_AMBLE_PRE, ETH_AMBLE_PRE, ETH_AMBLE_PRE,
    ETH_AMBLE_PRE, ETH_AMBLE_PRE, ETH_AMBLE_PRE, ETH_AMBLE_SOF);

-- 宛先をブロードキャストにし、スイッチが送信元のポート以外の全てへ送るようにする。
-- 送信元の MAC アドレスは、送り出すポートごとに変える。
-- ビットの並びの誤りがどこでも見つかるよう、ペイロードには連番を入れる。
function make_frame(src : mac_addr_t) return byte_array_t is
    constant header : std_logic_vector(111 downto 0) := MAC_ADDR_BROADCAST & src & ETHERTYPE;
    variable frame  : byte_array_t(0 to FRAME_BYTES + FCS_BYTES - 1) := (others => (others => '0'));
    variable crc    : crc_word_t := CRC_INIT;
begin
    for n in 0 to 13 loop
        frame(n) := header(111 - 8*n downto 104 - 8*n);
    end loop;
    for n in 14 to FRAME_BYTES - 1 loop
        frame(n) := std_logic_vector(to_unsigned(n, 8));
    end loop;
    for n in 0 to FRAME_BYTES - 1 loop
        crc := crc_next(crc, frame(n));
    end loop;
    for n in 0 to FCS_BYTES - 1 loop
        frame(FRAME_BYTES + n) := not flip_byte(crc(31 - 8*n downto 24 - 8*n));
    end loop;
    return frame;
end function;

-- 受け取ったバイト列から SFD の後ろを取り出し、送ったフレームと比べる。
function frame_matches(buf : byte_array_t; len : natural; frame : byte_array_t) return boolean is
    variable sof : integer := -1;
begin
    for n in 0 to len - 1 loop
        if buf(n) = ETH_AMBLE_SOF then
            sof := n;
            exit;
        end if;
    end loop;
    if sof < 0 or len - sof - 1 /= frame'length then
        return false;
    end if;
    for n in 0 to frame'length - 1 loop
        if buf(sof + 1 + n) /= frame(n) then
            return false;
        end if;
    end loop;
    return true;
end function;

-- ポートの番号は、0 が RGMII で、1 から RMII が並ぶ。
subtype port_t is natural range 0 to RMII_COUNT;
constant PORT_RGMII : port_t := 0;

type frame_list_t is array(port_t) of byte_array_t(0 to FRAME_BYTES + FCS_BYTES - 1);

function make_frames return frame_list_t is
    variable frames : frame_list_t;
begin
    for n in port_t loop
        frames(n) := make_frame(x"0200000000" & std_logic_vector(to_unsigned(n + 1, 8)));
    end loop;
    return frames;
end function;

constant FRAMES : frame_list_t := make_frames;

-- 受け取ったフレームが、どのポートから送ったものかを返す。どれとも一致しなければ -1 を返す。
function source_of(buf : byte_array_t; len : natural) return integer is
begin
    for n in port_t loop
        if frame_matches(buf, len, FRAMES(n)) then
            return n;
        end if;
    end loop;
    return -1;
end function;

-- 送信側のポートごとに、どのポートから送ったフレームを何回受けたかを数える。
type count_t is array(port_t) of natural;
type count_list_t is array(port_t) of count_t;

-- 送り出したポート以外の全てから、フレームが出たかを返す。
function flooded(got : count_list_t; src : port_t) return boolean is
begin
    for n in port_t loop
        if n /= src and got(n)(src) = 0 then
            return false;
        end if;
    end loop;
    return true;
end function;

signal phy_clk      : std_logic := '0';
signal sys_clk      : std_logic := '0';
signal rmii_clk     : std_logic_vector(RMII_COUNT-1 downto 0) := (others => '0');
signal pushbutton3  : std_logic := '1';

-- 信号名は FTDI から見た向きで、TXD_UART はホストが送る線、RXD_UART はコアが送る線である。
signal txd_uart     : std_logic := '1';
signal rxd_uart     : std_logic;

signal rgmii_txclk  : std_logic;
signal rgmii_txctrl : std_logic;
signal rgmii_txd    : std_logic_vector(3 downto 0);
signal rgmii_rxclk  : std_logic := '0';
signal rgmii_rxctrl : std_logic := '0';
signal rgmii_rxd    : std_logic_vector(3 downto 0) := RGMII_INBAND;
signal mdio_clk     : std_logic;
signal mdio_data    : std_logic;
signal phy_rst_n    : std_logic;

signal rmii_txd     : std_logic_vector(2*RMII_COUNT-1 downto 0);
signal rmii_txen    : std_logic_vector(RMII_COUNT-1 downto 0);
signal rmii_rxd     : std_logic_vector(2*RMII_COUNT-1 downto 0) := (others => '0');
signal rmii_crs_dv  : std_logic_vector(RMII_COUNT-1 downto 0) := (others => '0');

signal led          : std_logic_vector(7 downto 0);
signal got          : count_list_t := (others => (others => 0));

signal test_done    : boolean := false;

begin

uut : entity work.managed_switch
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
    rmii_ref_clk    => rmii_clk,
    rmii_txd        => rmii_txd,
    rmii_txen       => rmii_txen,
    rmii_rxd        => rmii_rxd,
    rmii_crs_dv     => rmii_crs_dv,
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

gen_rmii_clk : for n in 0 to RMII_COUNT-1 generate
    p_rmii_clk : process
    begin
        wait for n * RMII_PHASE_STEP;
        while not test_done loop
            rmii_clk(n) <= '0';
            wait for RMII_CLK_PERIOD / 2;
            rmii_clk(n) <= '1';
            wait for RMII_CLK_PERIOD / 2;
        end loop;
        wait;
    end process;
end generate;

p_rgmii_clk : process
begin
    while not test_done loop
        rgmii_rxclk <= '0';
        wait for RGMII_CLK_PERIOD / 2;
        rgmii_rxclk <= '1';
        wait for RGMII_CLK_PERIOD / 2;
    end loop;
    wait;
end process;

-- DP83867 と同じく、TXC の立ち上がりから RGMII_SKEW 後に取り込む。
-- 100 Mbps では 1 周期に 4 ビットを下位から送る。
p_rgmii_mon : process
    variable buf    : byte_array_t(0 to MAX_BYTES - 1);
    variable len    : natural := 0;
    variable low    : std_logic_vector(3 downto 0);
    variable half   : boolean := false;
    variable src    : integer;
begin
    wait until rising_edge(rgmii_txclk);
    wait for RGMII_SKEW;
    if rgmii_txctrl = '1' then
        if half then
            buf(len) := rgmii_txd & low;
            len := len + 1;
        else
            low := rgmii_txd;
        end if;
        half := not half;
    elsif len > 0 then
        src := source_of(buf, len);
        assert src >= 0
            report "RGMII sent a frame that matches none of the frames sent in" severity error;
        if src >= 0 then
            got(PORT_RGMII)(src) <= got(PORT_RGMII)(src) + 1;
        end if;
        len  := 0;
        half := false;
    end if;
end process;

-- LAN8720 と同じく、REF_CLK の立ち上がりで取り込む。
-- 100 Mbps では 1 周期に 2 ビットを下位から送る。
gen_rmii_mon : for n in 0 to RMII_COUNT-1 generate
    p_rmii_mon : process
        variable buf    : byte_array_t(0 to MAX_BYTES - 1);
        variable len    : natural := 0;
        variable byte   : byte_t := (others => '0');
        variable bits   : natural := 0;
        variable src    : integer;
    begin
        wait until rising_edge(rmii_clk(n));
        if rmii_txen(n) = '1' then
            byte := rmii_txd(2*n+1 downto 2*n) & byte(7 downto 2);
            bits := bits + 2;
            if bits = 8 then
                buf(len) := byte;
                len  := len + 1;
                bits := 0;
            end if;
        elsif len > 0 then
            src := source_of(buf, len);
            assert src >= 0
                report "RMII " & integer'image(n) & " sent a frame that matches none of the frames sent in"
                severity error;
            if src >= 0 then
                got(n + 1)(src) <= got(n + 1)(src) + 1;
            end if;
            len  := 0;
            bits := 0;
        end if;
    end process;
end generate;

p_test : process
    -- 100 Mbps の RGMII は、TXC の両方のエッジで同じ 4 ビットを送る。
    -- 立ち下がりの後で変え、次の立ち上がりと立ち下がりで同じ値を取り込ませる。
    procedure send_rgmii(frame : byte_array_t) is
        constant bytes : byte_array_t := PREAMBLE & frame;
    begin
        for n in bytes'range loop
            for half in 0 to 1 loop
                wait until falling_edge(rgmii_rxclk);
                wait for RGMII_SKEW;
                rgmii_rxd    <= bytes(n)(4*half + 3 downto 4*half);
                rgmii_rxctrl <= '1';
            end loop;
        end loop;
        wait until falling_edge(rgmii_rxclk);
        wait for RGMII_SKEW;
        rgmii_rxd    <= RGMII_INBAND;
        rgmii_rxctrl <= '0';
    end procedure;

    -- rising_edge は添字が変数の信号を受け取れないため、クロックの束の変化から立ち上がりを探す。
    procedure wait_rmii_rise(idx : natural) is
        variable prev : std_logic;
    begin
        loop
            prev := rmii_clk(idx);
            wait on rmii_clk;
            exit when prev = '0' and rmii_clk(idx) = '1';
        end loop;
    end procedure;

    procedure send_rmii(idx : natural; frame : byte_array_t) is
        constant bytes : byte_array_t := PREAMBLE & frame;
    begin
        for n in bytes'range loop
            for pair in 0 to 3 loop
                wait_rmii_rise(idx);
                wait for RMII_RX_DELAY;
                rmii_rxd(2*idx+1 downto 2*idx) <= bytes(n)(2*pair + 1 downto 2*pair);
                rmii_crs_dv(idx) <= '1';
            end loop;
        end loop;
        wait_rmii_rise(idx);
        wait for RMII_RX_DELAY;
        rmii_rxd(2*idx+1 downto 2*idx) <= "00";
        rmii_crs_dv(idx) <= '0';
    end procedure;
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
    assert rmii_txen = (rmii_txen'range => '0')
        report "reset: RMII is transmitting" severity error;

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

    -- 各ポートからフレームを 1 つずつ送り、送信元以外の全てのポートから 1 回ずつ出ることを確かめる。
    for src in port_t loop
        if src = PORT_RGMII then
            send_rgmii(FRAMES(src));
        else
            send_rmii(src - 1, FRAMES(src));
        end if;
        wait until flooded(got, src) for FRAME_TIMEOUT;
        for n in port_t loop
            if n /= src then
                assert got(n)(src) = 1
                    report "frame from port " & integer'image(src) & " was not sent once on port " & integer'image(n)
                    severity error;
            end if;
        end loop;
    end loop;

    -- スイッチは、フレームを送信元のポートへは返さない。
    for n in port_t loop
        assert got(n)(n) = 0
            report "port " & integer'image(n) & " sent a frame back to where it came from" severity error;
    end loop;

    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end sim;
