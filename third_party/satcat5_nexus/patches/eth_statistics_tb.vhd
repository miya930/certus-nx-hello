library ieee;
use     ieee.std_logic_1164.all;
use     ieee.numeric_std.all;
use     work.eth_frame_common.all;
use     work.switch_types.all;

-- eth_statistics のエラーの数を確かめる。
-- 統計のクロックと送信のクロックは、基板と同じく別の周波数と位相にする。
entity eth_statistics_tb is
end eth_statistics_tb;

architecture sim of eth_statistics_tb is

-- 統計は ConfigBus の 25 MHz で、送信は RGMII の 125 MHz で動く。
constant STATUS_CLK_PERIOD : time := 1 sec / 25_000_000;
constant TX_CLK_PERIOD     : time := 1 sec / 125_000_000;
-- 2 つのクロックは別の発振器から来るため、端をそろえない。
constant TX_CLK_PHASE      : time := 3 ns;
-- トグルがクロックの境を越えて届くまで待つ時間。
constant SETTLE            : time := 10 * STATUS_CLK_PERIOD;
constant REFRESH_COUNT     : positive := 8;

signal status_clk  : std_logic := '0';
signal tx_clk      : std_logic := '0';
signal test_done   : boolean := false;

signal stats_req_t : std_logic := '0';
signal err_port    : port_error_t := PORT_ERROR_NONE;
signal err_mii     : byte_u;
signal err_pkt     : byte_u;

begin

status_clk <= not status_clk after STATUS_CLK_PERIOD / 2 when not test_done;

p_tx_clk : process
begin
    wait for TX_CLK_PHASE;
    while not test_done loop
        tx_clk <= not tx_clk;
        wait for TX_CLK_PERIOD / 2;
    end loop;
    wait;
end process;

u_stats : entity work.eth_statistics
    generic map(
    IO_BYTES    => 1,
    COUNT_WIDTH => 32,
    SAFE_COUNT  => true)
    port map(
    stats_req_t => stats_req_t,
    bcst_bytes  => open,
    bcst_frames => open,
    rcvd_bytes  => open,
    rcvd_frames => open,
    sent_bytes  => open,
    sent_frames => open,
    delta_freq  => open,
    port_rate   => get_rate_word(100),
    port_status => (others => '0'),
    status_clk  => status_clk,
    status_word => open,
    err_port    => err_port,
    err_mii     => err_mii,
    err_ovr_tx  => open,
    err_ovr_rx  => open,
    err_pkt     => err_pkt,
    err_ptp_tx  => open,
    err_ptp_rx  => open,
    rx_reset_p  => '0',
    rx_clk      => tx_clk,
    rx_data     => (others => '0'),
    rx_nlast    => 0,
    rx_write    => '0',
    tx_reset_p  => '0',
    tx_clk      => tx_clk,
    tx_nlast    => 0,
    tx_write    => '0');

p_test : process
    -- 取り込みを求め、前回の取り込みからの数がエラーの出力に出るまで待つ。
    procedure refresh is
    begin
        stats_req_t <= not stats_req_t;
        wait for SETTLE;
    end procedure;
begin
    wait for SETTLE;
    refresh;

    -- MAC/PHY のエラーを 2 件、パケットのエラーを 1 件起こす。
    -- エラーは、トグルが変わるたびに 1 件と数える。
    err_port.mii_err <= not err_port.mii_err;
    wait for SETTLE;
    err_port.mii_err <= not err_port.mii_err;
    wait for SETTLE;
    err_port.pkt_err <= not err_port.pkt_err;
    wait for SETTLE;

    refresh;
    assert err_mii = 2
        report "MAC/PHY errors: expected 2, got " & integer'image(to_integer(err_mii)) severity error;
    assert err_pkt = 1
        report "packet errors: expected 1, got " & integer'image(to_integer(err_pkt)) severity error;

    -- 新しいエラーがなければ、以後の取り込みは毎回 0 を報告する。
    for n in 1 to REFRESH_COUNT loop
        refresh;
        assert err_mii = 0 and err_pkt = 0
            report "error counters were not cleared by refresh " & integer'image(n) severity error;
    end loop;

    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end sim;
