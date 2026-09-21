library ieee;
use     ieee.std_logic_1164.all;

entity riscv_rust_tb is
end riscv_rust_tb;

architecture sim of riscv_rust_tb is

constant CLK_HZ     : positive := 25_000_000;
constant CLK_PERIOD : time := 1 sec / CLK_HZ;

-- NEORV32 は TCK をコアのクロックで取り込むため、TCK はコアのクロックの 1/5 以下にする。
constant TCK_PERIOD : time := 10 * CLK_PERIOD;

-- 版と部品番号は 0 で、製造元の欄は Lattice の 0x021、最下位ビットは常に 1 である。
constant IDCODE : std_logic_vector(31 downto 0) := x"00000043";

signal clk          : std_logic := '0';
signal pushbutton3  : std_logic := '1';
-- 信号名は FTDI から見た向きで、TXD_UART はホストが送る線、RXD_UART はコアが送る線である。
signal txd_uart     : std_logic := '1';
signal rxd_uart     : std_logic;
signal jtag_tck     : std_logic := '0';
signal jtag_tms     : std_logic := '1';
signal jtag_tdi     : std_logic := '1';
signal jtag_tdo     : std_logic;
signal led          : std_logic_vector(7 downto 0);
signal test_done    : boolean := false;

begin

uut : entity work.riscv_rust
    generic map(
    CLK_HZ => CLK_HZ)
    port map(
    system_25m_clk  => clk,
    pushbutton3     => pushbutton3,
    rxd_uart        => rxd_uart,
    txd_uart        => txd_uart,
    jtag_tck        => jtag_tck,
    jtag_tms        => jtag_tms,
    jtag_tdi        => jtag_tdi,
    jtag_tdo        => jtag_tdo,
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

p_test : process
    -- TCK を 1 周期動かし、立ち上がりで TDO を取り込む。
    procedure jtag_clock(tms : std_logic; tdo : out std_logic) is
    begin
        jtag_tms <= tms;
        wait for TCK_PERIOD / 2;
        jtag_tck <= '1';
        tdo := jtag_tdo;
        wait for TCK_PERIOD / 2;
        jtag_tck <= '0';
    end procedure;

    variable tdo    : std_logic;
    variable dr     : std_logic_vector(31 downto 0);
begin
    -- 押しボタンを押した状態から始める。
    pushbutton3 <= '0';
    for n in 1 to 10 loop
        wait until rising_edge(clk);
    end loop;

    assert led = (led'range => '1')
        report "reset: an LED is lit" severity error;
    assert rxd_uart = '1'
        report "reset: UART is not idle" severity error;

    pushbutton3 <= '1';
    wait until rising_edge(clk);

    -- TAP をリセットすると、命令レジスタは IDCODE を選ぶ。
    -- Test-Logic-Reset から Run-Test/Idle、Select-DR-Scan、Capture-DR を経て Shift-DR に入る。
    for n in 1 to 5 loop
        jtag_clock('1', tdo);
    end loop;
    jtag_clock('0', tdo);
    jtag_clock('1', tdo);
    jtag_clock('0', tdo);
    jtag_clock('0', tdo);

    -- 最下位ビットから読み、最後のビットで Exit1-DR に抜ける。
    for n in dr'reverse_range loop
        if n = dr'high then
            jtag_clock('1', dr(n));
        else
            jtag_clock('0', dr(n));
        end if;
    end loop;
    jtag_clock('1', tdo);
    jtag_clock('0', tdo);

    assert dr = IDCODE
        report "JTAG: IDCODE does not match" severity error;

    report "All tests finished.";
    test_done <= true;
    wait;
end process;

end sim;
