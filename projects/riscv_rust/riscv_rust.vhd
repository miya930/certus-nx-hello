library ieee;
use     ieee.std_logic_1164.all;

library neorv32;

entity riscv_rust is
    generic (
    CLK_HZ      : positive := 25_000_000;
    IMEM_BYTES  : positive := 16 * 1024;
    DMEM_BYTES  : positive := 8 * 1024);
    port (
    system_25m_clk  : in  std_logic;
    pushbutton3     : in  std_logic;
    txd_uart        : in  std_logic;
    rxd_uart        : out std_logic;
    jtag_tck        : in  std_logic;
    jtag_tms        : in  std_logic;
    jtag_tdi        : in  std_logic;
    jtag_tdo        : out std_logic;
    led             : out std_logic_vector(7 downto 0));
end riscv_rust;

architecture rtl of riscv_rust is

-- probe-rs は、JTAG の IDCODE の製造元の欄が 0 だと無効として接続しない。
-- NEORV32 は自身の JEDEC ID を持たないため、コアが載る FPGA の製造元である Lattice の値を使う。
-- この値は、FPGA 自身が返す IDCODE 0x310F1043 の製造元の欄と同じである。
constant LATTICE_JEDEC_ID : std_ulogic_vector(10 downto 0) := "00000100001";

signal gpio : std_ulogic_vector(31 downto 0);

begin

-- 押しボタンは押している間だけ 0 になり、コアのリセットも負論理なので、そのまま渡す。
-- 起動 ROM は、JTAG から命令メモリに書き込まれたファームウェアへ飛ぶ。
-- UART の信号名は FTDI から見た向きで、TXD_UART は FTDI が送る線、RXD_UART は FTDI が受ける線である。
-- コアから見ると送受が入れ替わる。
u_core : entity neorv32.neorv32_top
    generic map(
    CLOCK_FREQUENCY  => CLK_HZ,
    BOOT_MODE_SELECT => 0,
    OCD_EN           => true,
    OCD_JEDEC_ID     => LATTICE_JEDEC_ID,
    RISCV_ISA_C      => true,
    RISCV_ISA_M      => true,
    RISCV_ISA_Zicntr => true,
    IMEM_EN          => true,
    IMEM_SIZE        => IMEM_BYTES,
    DMEM_EN          => true,
    DMEM_SIZE        => DMEM_BYTES,
    IO_GPIO_NUM      => led'length,
    IO_CLINT_EN      => true,
    IO_UART0_EN      => true)
    port map(
    clk_i       => system_25m_clk,
    rstn_i      => pushbutton3,
    gpio_o      => gpio,
    uart0_txd_o => rxd_uart,
    uart0_rxd_i => txd_uart,
    jtag_tck_i  => jtag_tck,
    jtag_tms_i  => jtag_tms,
    jtag_tdi_i  => jtag_tdi,
    jtag_tdo_o  => jtag_tdo);

-- LED は、出力を 0 にすると点灯する。
led <= not std_logic_vector(gpio(led'range));

end rtl;
