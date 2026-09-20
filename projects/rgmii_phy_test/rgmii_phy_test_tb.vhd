library ieee;
use     ieee.numeric_std.all;
use     ieee.std_logic_1164.all;
use     work.eth_frame_common.all;

entity rgmii_phy_test_tb is
end rgmii_phy_test_tb;

architecture tb of rgmii_phy_test_tb is

constant CLK_PERIOD     : time := 8 ns;

-- 起動の待ち時間は、確認に必要な長さまで短くする。
constant PHY_ADDR       : natural := 3;
constant RESET_USEC     : positive := 2;
constant STARTUP_USEC   : positive := 10;

-- rgmii_phy_test と同じ値にする。
constant TEST_DST       : mac_addr_t := x"FFFFFFFFFFFF";
constant TEST_SRC       : mac_addr_t := x"5A5A00000001";
constant TEST_ETYPE     : mac_type_t := x"88B5";
constant TEST_NBYTES    : positive := 256;
constant FRAME_LEN      : positive := 14 + TEST_NBYTES + 4;

constant PREAMBLE_BYTE  : byte_t := x"55";
constant SFD_BYTE       : byte_t := x"D5";

-- MDIO で書き込むはずのレジスタと値。
type mdio_cmd_t is record
    reg     : natural;
    value   : natural;
end record;
type mdio_cmd_array_t is array(natural range <>) of mdio_cmd_t;
constant EXPECT_CMDS : mdio_cmd_array_t := (
    (16#0D#, 16#001F#), (16#0E#, 16#0032#), (16#0D#, 16#401F#), (16#0E#, 16#00D3#),
    (16#0D#, 16#001F#), (16#0E#, 16#0086#), (16#0D#, 16#401F#), (16#0E#, 16#0077#));

type frame_t is array(0 to FRAME_LEN-1) of byte_t;

-- LSB から順に処理する CRC-32。戻り値の下位バイトから順に FCS として送る。
function eth_crc(frm : frame_t; len : natural) return crc_word_t is
    variable crc : crc_word_t := CRC_INIT;
begin
    for i in 0 to len-1 loop
        for b in 0 to 7 loop
            if (crc(0) xor frm(i)(b)) = '1' then
                crc := ('0' & crc(31 downto 1)) xor x"EDB88320";
            else
                crc := '0' & crc(31 downto 1);
            end if;
        end loop;
    end loop;
    return not crc;
end function;

-- PHY は、リンクが上がると 125 MHz の受信クロックを出し続ける。
-- フレームを送っていない間は、受信データにリンクの状態と速度を載せる。
constant RX_IDLE_META   : std_logic_vector(3 downto 0) := "1100";

signal clk              : std_logic := '0';
signal rx_clk           : std_logic := '0';
signal pushbutton3      : std_logic := '0';
signal rgmii_txclk      : std_logic;
signal rgmii_txctrl     : std_logic;
signal rgmii_txd        : std_logic_vector(3 downto 0);
signal rgmii_mdio_clk   : std_logic;
signal rgmii_mdio_data  : std_logic;
signal rgmii_rst_n      : std_logic;
signal led              : std_logic_vector(1 downto 0);

signal mdio_done        : boolean := false;
signal frames_done      : boolean := false;
signal test_done        : boolean := false;

begin

uut : entity work.rgmii_phy_test
    generic map(
    PHY_ADDR        => PHY_ADDR,
    RESET_USEC      => RESET_USEC,
    STARTUP_USEC    => STARTUP_USEC)
    port map(
    rgmii_txclk     => rgmii_txclk,
    rgmii_txctrl    => rgmii_txctrl,
    rgmii_txd       => rgmii_txd,
    rgmii_rxclk     => rx_clk,
    rgmii_rxctrl    => '0',
    rgmii_rxd       => RX_IDLE_META,
    rgmii_mdio_clk  => rgmii_mdio_clk,
    rgmii_mdio_data => rgmii_mdio_data,
    rgmii_rst_n     => rgmii_rst_n,
    clk_customer2   => clk,
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

-- PHY の受信クロックは、FPGA のクロックとは別の発振器から作られるため、位相をずらす。
p_rx_clk : process
begin
    wait for CLK_PERIOD / 4;
    while not test_done loop
        rx_clk <= '0';
        wait for CLK_PERIOD / 2;
        rx_clk <= '1';
        wait for CLK_PERIOD / 2;
    end loop;
    wait;
end process;

-- 押しボタンは、押している間だけ 0 になる。
p_reset : process
begin
    pushbutton3 <= '0';
    wait for 10 * CLK_PERIOD;
    pushbutton3 <= '1';
    wait;
end process;

-- PHY のリセットは、押しボタンを離してから解除され、その後は解除されたままになる。
p_phy_reset : process
    variable released : time;
begin
    wait until pushbutton3 = '1';
    released := now;
    wait for CLK_PERIOD;
    assert rgmii_rst_n = '0'
        report "PHY reset is not asserted at startup" severity error;
    wait until rgmii_rst_n = '1';
    assert now - released >= RESET_USEC * 1 us
        report "PHY reset pulse is too short" severity error;
    assert now - released <= (RESET_USEC + 1) * 1 us
        report "PHY reset is released too late" severity error;
    wait until rgmii_rst_n = '0' for 20 us;
    assert rgmii_rst_n = '1'
        report "PHY reset is asserted again" severity error;
    wait;
end process;

-- MDIO の書き込みを取り込み、PHY アドレスとレジスタと値を確かめる。
p_mdio : process
    variable ones   : natural := 0;
    variable bits   : std_logic_vector(31 downto 0);
    variable phyad  : natural;
    variable regad  : natural;
    variable value  : natural;
begin
    for n in EXPECT_CMDS'range loop
        -- プリアンブルは 1 が 32 個続き、その後の 0 が最初のフレームの先頭になる。
        ones := 0;
        loop
            wait until rising_edge(rgmii_mdio_clk);
            assert rgmii_rst_n = '1'
                report "MDIO starts before the PHY reset is released" severity error;
            if rgmii_mdio_data = '1' then
                ones := ones + 1;
            elsif ones >= 32 then
                exit;
            else
                ones := 0;
            end if;
        end loop;

        -- 先頭の 0 は ST の 1 ビット目にあたるため、残りの 31 ビットを取り込む。
        bits(31) := '0';
        for b in 30 downto 0 loop
            wait until rising_edge(rgmii_mdio_clk);
            bits(b) := rgmii_mdio_data;
        end loop;

        phyad := to_integer(unsigned(bits(27 downto 23)));
        regad := to_integer(unsigned(bits(22 downto 18)));
        value := to_integer(unsigned(bits(15 downto 0)));

        assert bits(31 downto 30) = "01"
            report "MDIO: wrong start bits" severity error;
        assert bits(29 downto 28) = "01"
            report "MDIO: not a write command" severity error;
        assert bits(17 downto 16) = "10"
            report "MDIO: wrong turnaround bits" severity error;
        assert phyad = PHY_ADDR
            report "MDIO: PHY address is " & integer'image(phyad) severity error;
        assert regad = EXPECT_CMDS(n).reg
            report "MDIO: register is " & integer'image(regad) severity error;
        assert value = EXPECT_CMDS(n).value
            report "MDIO: value of register " & integer'image(regad)
                 & " is " & integer'image(value) severity error;
    end loop;

    report "MDIO test finished.";
    mdio_done <= true;
    wait;
end process;

-- RGMII の送信を取り込み、テストフレームの中身を確かめる。
-- 送信データと TXC はどちらも同じクロックから出るため、TXC ではなくそのクロックで見る。
p_frames : process
    variable low    : std_logic_vector(3 downto 0);
    variable high   : std_logic_vector(3 downto 0);
    variable data   : std_logic;
    variable frame  : frame_t;
    variable len    : natural;
    variable crc    : crc_word_t;

    procedure next_byte(value : out byte_t; valid : out std_logic) is
    begin
        wait until rising_edge(clk);
        wait for CLK_PERIOD / 4;
        low := rgmii_txd;
        data := rgmii_txctrl;
        wait until falling_edge(clk);
        wait for CLK_PERIOD / 4;
        high := rgmii_txd;
        value := high & low;
        valid := data;
    end procedure;

    variable value : byte_t;
    variable valid : std_logic;
begin
    for n in 1 to 2 loop
        -- プリアンブルの先頭まで待つ。
        loop
            next_byte(value, valid);
            exit when valid = '1' and value = PREAMBLE_BYTE;
        end loop;

        -- プリアンブルの後に SFD が来る。
        loop
            next_byte(value, valid);
            assert valid = '1'
                report "frame ends during the preamble" severity error;
            exit when value /= PREAMBLE_BYTE;
        end loop;
        assert value = SFD_BYTE
            report "start of frame delimiter is missing" severity error;

        -- フレームの終わりまで取り込む。
        len := 0;
        loop
            next_byte(value, valid);
            exit when valid = '0';
            assert len < FRAME_LEN
                report "frame is longer than expected" severity error;
            if len < FRAME_LEN then
                frame(len) := value;
            end if;
            len := len + 1;
        end loop;

        assert len = FRAME_LEN
            report "frame length is " & integer'image(len) severity error;
        for i in 0 to 5 loop
            assert frame(i) = TEST_DST(47-8*i downto 40-8*i)
                report "wrong destination address" severity error;
            assert frame(6+i) = TEST_SRC(47-8*i downto 40-8*i)
                report "wrong source address" severity error;
        end loop;
        assert frame(12) & frame(13) = TEST_ETYPE
            report "wrong EtherType" severity error;

        crc := eth_crc(frame, FRAME_LEN-4);
        for i in 0 to 3 loop
            assert frame(FRAME_LEN-4+i) = crc(8*i+7 downto 8*i)
                report "wrong FCS" severity error;
        end loop;
    end loop;

    report "Frame test finished.";
    frames_done <= true;
    wait;
end process;

p_test : process
begin
    wait until mdio_done and frames_done;
    assert led(0) = '0'
        report "LED does not show that the MDIO writes finished" severity error;
    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end tb;
