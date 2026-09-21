library ieee;
use     ieee.std_logic_1164.all;
use     ieee.numeric_std.all;
use     work.common_functions.all;
use     work.eth_frame_common.all;
use     work.switch_types.all;

entity vlan_ptp_switch_tb is
end vlan_ptp_switch_tb;

architecture tb of vlan_ptp_switch_tb is

constant PORT_COUNT     : positive := 4;

-- vlan_ptp_switch と同じ値にする。
constant CORE_CLK_HZ    : natural := 50_000_000;
constant CFG_DEV_ADDR   : natural := 0;

-- port_cfgbus の既定の MAC アドレスと EtherType に合わせる。
constant CFG_MACADDR    : mac_addr_t := x"5A5ADEADBEEF";
constant CFG_ETYPE_CMD  : mac_type_t := x"5C01";
constant CFG_ETYPE_ACK  : mac_type_t := x"5C02";
constant CFG_OP_WRITE   : byte_t := x"2F";
constant CFG_OP_READ    : byte_t := x"40";

constant HOST_PORT      : natural := 0;
constant HOST_MACADDR   : mac_addr_t := x"020000000099";
constant UNKNOWN_MACADDR: mac_addr_t := x"0200000000EE";
constant PTP_MACADDR    : mac_addr_t := x"011B19000000";
constant VLAN_VID       : natural := 2;

constant FRAME_WAIT     : time := 300 us;
constant REPLY_TIMEOUT  : time := 1 ms;

-- FCS を除いた最小フレーム長。
constant FRAME_MIN_LEN  : positive := 60;
constant FRAME_MAX_LEN  : positive := 128;

type frame_t is array(0 to FRAME_MAX_LEN-1) of byte_t;
type frame_array_t is array(natural range <>) of frame_t;
type count_array_t is array(natural range <>) of natural;

function port_mac(n : natural) return mac_addr_t is
begin
    return x"0200000000" & std_logic_vector(to_unsigned(n + 1, 8));
end function;

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

signal ref_clk      : std_logic := '0';
signal sys_clk      : std_logic := '0';
signal reset_p      : std_logic := '1';
signal sim_done     : boolean := false;

signal dut_txd      : std_logic_vector(2*PORT_COUNT-1 downto 0);
signal dut_tx_en    : std_logic_vector(PORT_COUNT-1 downto 0);
signal dut_rxd      : std_logic_vector(2*PORT_COUNT-1 downto 0);
signal dut_crs_dv   : std_logic_vector(PORT_COUNT-1 downto 0);

-- DUT の各ポートにつなぐ相手側 MAC のストリーム。
signal rx_data      : array_rx_m2s(PORT_COUNT-1 downto 0);
signal tx_data      : array_tx_s2m(PORT_COUNT-1 downto 0) := (others => TX_S2M_IDLE);
signal tx_ctrl      : array_tx_m2s(PORT_COUNT-1 downto 0);

signal rx_frame     : frame_array_t(PORT_COUNT-1 downto 0);
signal rx_len       : count_array_t(PORT_COUNT-1 downto 0) := (others => 0);
signal rx_count     : count_array_t(PORT_COUNT-1 downto 0) := (others => 0);

begin

ref_clk <= not ref_clk after 10 ns when not sim_done else ref_clk;
sys_clk <= not sys_clk after 20 ns when not sim_done else sys_clk;

uut : entity work.vlan_ptp_switch
    generic map(
    PORT_COUNT      => PORT_COUNT)
    port map(
    rmii_txd        => dut_txd,
    rmii_tx_en      => dut_tx_en,
    rmii_rxd        => dut_rxd,
    rmii_crs_dv     => dut_crs_dv,
    rmii_ref_clk    => ref_clk,
    system_25m_clk  => sys_clk,
    reset_p         => reset_p);

gen_partner : for n in 0 to PORT_COUNT-1 generate
    u_partner : entity work.port_rmii
        generic map(
        MODE_CLKOUT => false)
        port map(
        rmii_txd    => dut_rxd(2*n+1 downto 2*n),
        rmii_txen   => dut_crs_dv(n),
        rmii_txer   => open,
        rmii_rxd    => dut_txd(2*n+1 downto 2*n),
        rmii_rxen   => dut_tx_en(n),
        rmii_rxer   => '0',
        rmii_clkin  => ref_clk,
        rmii_clkout => open,
        rx_data     => rx_data(n),
        tx_data     => tx_data(n),
        tx_ctrl     => tx_ctrl(n),
        lock_refclk => sys_clk,
        reset_p     => reset_p);

    p_rx : process(rx_data(n).clk)
        variable buf : frame_t := (others => (others => '0'));
        variable len : natural := 0;
    begin
        if rising_edge(rx_data(n).clk) then
            if (rx_data(n).write = '1') then
                if (len < FRAME_MAX_LEN) then
                    buf(len) := rx_data(n).data;
                end if;
                len := len + 1;
                if (rx_data(n).last = '1') then
                    rx_frame(n) <= buf;
                    rx_len(n)   <= len;
                    rx_count(n) <= rx_count(n) + 1;
                    len := 0;
                end if;
            end if;
        end if;
    end process;
end generate;

p_test : process
    variable frm        : frame_t;
    variable len        : natural;
    variable before     : count_array_t(PORT_COUNT-1 downto 0);
    variable cfg_seq    : natural := 0;
    variable reg_val    : std_logic_vector(31 downto 0);
    variable correction : std_logic_vector(63 downto 0);

    procedure put(x : std_logic_vector) is
        alias xa : std_logic_vector(x'length-1 downto 0) is x;
    begin
        for i in x'length/8-1 downto 0 loop
            frm(len) := xa(8*i+7 downto 8*i);
            len := len + 1;
        end loop;
    end procedure;

    procedure start_frame(dst, src : mac_addr_t; etype : mac_type_t) is
    begin
        frm := (others => (others => '0'));
        len := 0;
        put(dst);
        put(src);
        put(etype);
    end procedure;

    procedure put_fcs is
        variable fcs : crc_word_t;
    begin
        fcs := eth_crc(frm, len);
        put(fcs(7 downto 0));
        put(fcs(15 downto 8));
        put(fcs(23 downto 16));
        put(fcs(31 downto 24));
    end procedure;

    procedure send_frame(p : natural) is
    begin
        if (len < FRAME_MIN_LEN) then
            len := FRAME_MIN_LEN;
        end if;
        put_fcs;
        -- 相手側 MAC は全て同じ REF_CLK で動くため、ポート 0 のクロックで待つ。
        -- クロックの立ち上がりと同じ時刻に VALID を変えると先頭バイトを取りこぼすため、先に立ち上がりを待つ。
        wait until rising_edge(tx_ctrl(0).clk);
        for i in 0 to len-1 loop
            tx_data(p).data  <= frm(i);
            tx_data(p).last  <= bool2bit(i = len-1);
            tx_data(p).valid <= '1';
            wait until rising_edge(tx_ctrl(0).clk) and tx_ctrl(p).ready = '1';
        end loop;
        tx_data(p) <= TX_S2M_IDLE;
    end procedure;

    -- expect(n) = '1' のポートにだけフレームが 1 つ届いたことを確認する。
    procedure check_delivery(name : string; expect : std_logic_vector(PORT_COUNT-1 downto 0)) is
    begin
        wait for FRAME_WAIT;
        for n in 0 to PORT_COUNT-1 loop
            if (expect(n) = '1') then
                assert (rx_count(n) = before(n) + 1)
                    report name & ": port " & integer'image(n) & " did not receive the frame"
                    severity error;
            else
                assert (rx_count(n) = before(n))
                    report name & ": port " & integer'image(n) & " received an unexpected frame"
                    severity error;
            end if;
        end loop;
    end procedure;

    procedure check_fcs(n : natural) is
        variable rx       : frame_t;
        variable l        : natural;
        variable fcs      : crc_word_t;
        variable received : crc_word_t;
    begin
        rx       := rx_frame(n);
        l        := rx_len(n);
        if (l > 4) then
            fcs      := eth_crc(rx, l-4);
            received := rx(l-1) & rx(l-2) & rx(l-3) & rx(l-4);
            assert (received = fcs)
                report "port " & integer'image(n) & ": FCS mismatch" severity error;
        end if;
    end procedure;

    -- ConfigBus のコマンドは、ポート 0 につないだ PC から Ethernet フレームで送る。
    procedure cfg_command(opcode : byte_t; regaddr : natural; wdata : std_logic_vector(31 downto 0)) is
        variable count_before : natural;
    begin
        start_frame(CFG_MACADDR, HOST_MACADDR, CFG_ETYPE_CMD);
        put(opcode);
        put(x"00");
        put(std_logic_vector(to_unsigned(cfg_seq mod 256, 8)));
        put(x"00");
        put(std_logic_vector(to_unsigned(CFG_DEV_ADDR * 1024 + regaddr, 32)));
        if (opcode = CFG_OP_WRITE) then
            put(wdata);
        end if;
        cfg_seq := cfg_seq + 1;

        count_before := rx_count(HOST_PORT);
        send_frame(HOST_PORT);
        loop
            wait until (rx_count(HOST_PORT) /= count_before) for REPLY_TIMEOUT;
            assert (rx_count(HOST_PORT) /= count_before)
                report "ConfigBus: no reply" severity failure;
            exit when (rx_frame(HOST_PORT)(12) & rx_frame(HOST_PORT)(13) = CFG_ETYPE_ACK);
            count_before := rx_count(HOST_PORT);
        end loop;
    end procedure;

    procedure cfg_write(regaddr : natural; wdata : std_logic_vector(31 downto 0)) is
    begin
        cfg_command(CFG_OP_WRITE, regaddr, wdata);
    end procedure;

    procedure cfg_read(regaddr : natural; expected : natural) is
    begin
        cfg_command(CFG_OP_READ, regaddr, (others => '0'));
        reg_val := rx_frame(HOST_PORT)(22) & rx_frame(HOST_PORT)(23) & rx_frame(HOST_PORT)(24) & rx_frame(HOST_PORT)(25);
        assert (rx_frame(HOST_PORT)(26) = x"00")
            report "ConfigBus: read error flag on register " & integer'image(regaddr) severity error;
        assert (unsigned(reg_val) = expected)
            report "ConfigBus: register " & integer'image(regaddr) & " = "
                 & integer'image(to_integer(unsigned(reg_val))) & ", expected " & integer'image(expected)
            severity error;
    end procedure;

begin
    reset_p <= '1';
    wait for 200 us;
    reset_p <= '0';
    for n in 0 to PORT_COUNT-1 loop
        if (rx_data(n).reset_p = '1') then
            wait until (rx_data(n).reset_p = '0');
        end if;
    end loop;

    -- 読み出し専用レジスタから、スイッチの構成が読める。ポート数は ConfigBus 用の内部ポートを含む。
    cfg_read(SW_ADDR_PORT_COUNT, PORT_COUNT + 1);
    cfg_read(SW_ADDR_DATA_WIDTH, 8);
    cfg_read(SW_ADDR_CORE_CLOCK, CORE_CLK_HZ);
    report "ConfigBus read test finished.";

    -- 未学習の宛先へのフレームは、送信元以外の全ポートへ送られる。
    before := rx_count;
    start_frame(UNKNOWN_MACADDR, port_mac(0), ETYPE_IPV4);
    send_frame(0);
    check_delivery("Unknown destination", "1110");
    report "Unknown destination test finished.";

    -- 送信元として学習した MAC アドレス宛てのフレームは、そのポートにだけ送られる。
    before := rx_count;
    start_frame(port_mac(0), port_mac(2), ETYPE_IPV4);
    send_frame(2);
    check_delivery("Learned destination", "0001");
    assert (rx_frame(0)(0) & rx_frame(0)(1) & rx_frame(0)(2) & rx_frame(0)(3) & rx_frame(0)(4) & rx_frame(0)(5) = port_mac(0))
        report "Learned destination: wrong destination address" severity error;
    report "Learned destination test finished.";

    -- PTP の Sync は、スイッチ内の滞在時間を correctionField に加えて送られる。
    before := rx_count;
    start_frame(PTP_MACADDR, port_mac(1), ETYPE_PTP);
    put(x"00");                 -- messageType = Sync
    put(x"02");                 -- versionPTP = 2
    put(x"002C");               -- messageLength
    put(x"00");                 -- domainNumber
    put(x"00");                 -- minorSdoId
    put(x"0000");               -- flagField
    put(x"0000000000000000");   -- correctionField
    len := ETH_HDR_DATA + 44;
    send_frame(1);
    check_delivery("PTP Sync", "1101");
    for n in 0 to PORT_COUNT-1 loop
        if (n /= 1) then
            check_fcs(n);
            for i in 0 to 7 loop
                correction(63-8*i downto 56-8*i) := rx_frame(n)(ETH_HDR_DATA + 8 + i);
            end loop;
            assert (correction(63) = '0' and unsigned(correction) /= 0)
                report "PTP Sync: port " & integer'image(n) & " correctionField was not increased"
                severity error;
        end if;
    end loop;
    report "PTP Sync test finished.";

    -- VID ごとのポートマスクに含まれないポートへは、そのタグのフレームを送らない。
    -- 出力ポートの既定の設定ではタグを付けないため、受信側ではタグが取り除かれている。
    cfg_write(SW_ADDR_VLAN_VID, std_logic_vector(to_unsigned(VLAN_VID, 32)));
    cfg_write(SW_ADDR_VLAN_MASK, x"00000003");
    before := rx_count;
    start_frame(MAC_ADDR_BROADCAST, port_mac(0), ETYPE_VLAN);
    put(std_logic_vector(to_unsigned(VLAN_VID, 16)));
    put(ETYPE_IPV4);
    send_frame(0);
    check_delivery("VLAN tagged", "0010");
    assert (rx_frame(1)(12) & rx_frame(1)(13) = ETYPE_IPV4)
        report "VLAN tagged: tag was not removed" severity error;

    before := rx_count;
    start_frame(MAC_ADDR_BROADCAST, port_mac(0), ETYPE_IPV4);
    send_frame(0);
    check_delivery("VLAN untagged", "1110");
    report "VLAN test finished.";

    report "All tests finished.";
    sim_done <= true;
    wait;
end process;

end tb;
