DEVICE  := LFD2NX-40-8BG256C
NEORV32 := ../../third_party/neorv32
BUILD   := build
PORT    ?= /dev/ttyUSB1

# llvm-objcopy はホスト以外の対象を扱えるため、RISC-V の ELF を平坦なバイナリにできる。
LLVM_OBJCOPY := $(shell find $$(rustc --print sysroot) -name llvm-objcopy | head -1)
FIRMWARE_ELF := $(BUILD)/firmware/riscv32imc-unknown-none-elf/release/riscv_rust_firmware
EXE          := $(BUILD)/neorv32_exe.bin

# NEORV32 は VHDL-2008 で書かれ、自身を neorv32 ライブラリに置くことを前提にしている。
# トップも同じライブラリに入れると、ライブラリ参照が 1 つで済む。
GHDL_FLAGS := --std=08 --work=neorv32

# NEORV32 が file_list_core.f に解析順を持っているため、GHDL に解析順を求めない。
CORE_VHDL := $(shell sed 's|$$NEORV32_HOME|$(NEORV32)|g' $(NEORV32)/rtl/file_list_core.f)
VHDL := $(CORE_VHDL) riscv_rust.vhd

all: $(BUILD)/riscv_rust.bit

# 解析から合成までを yosys の 1 プロセスで行うため、解析結果をライブラリに残さない。
$(BUILD)/riscv_rust.json: $(VHDL) Makefile
	mkdir -p $(BUILD)
	yosys -m ghdl -l $(BUILD)/yosys.log -q -p \
		"ghdl $(GHDL_FLAGS) $(VHDL) -e riscv_rust; \
		 synth_nexus -top riscv_rust -json $@"

# PDC の create_clock は入力バッファより先のクロック網に伝わらないため、目標の周波数は nextpnr-nexus に渡す。
$(BUILD)/riscv_rust.fasm: $(BUILD)/riscv_rust.json riscv_rust.pdc
	nextpnr-nexus --device $(DEVICE) --pdc riscv_rust.pdc --freq 25 --json $< \
		--fasm $@ --report $(BUILD)/report.json -l $(BUILD)/nextpnr.log -q

$(BUILD)/riscv_rust.bit: $(BUILD)/riscv_rust.fasm
	prjoxide pack $< $@

SIM_VHDL := $(VHDL) riscv_rust_tb.vhd

# ghdl -c は解析結果をライブラリに保存しないため、システム時刻が巻き戻っても失敗しない。
# コアのメモリは初期値を持たないまま読み出されることがあり、
# IEEE のライブラリが metavalue の警告を出すため、その警告は止める。
sim: $(SIM_VHDL)
	mkdir -p build/sim
	ghdl -c $(GHDL_FLAGS) --workdir=build/sim -o build/sim/riscv_rust_tb $(SIM_VHDL) -e riscv_rust_tb
	build/sim/riscv_rust_tb --assert-level=error --ieee-asserts=disable

# SRAM に書き込む。電源を切ると消える。
load: $(BUILD)/riscv_rust.bit
	openFPGALoader -b certusnx_versa_evn $<

# SPI Flash に書き込む。電源を入れ直しても残る。
flash: $(BUILD)/riscv_rust.bit
	openFPGALoader -b certusnx_versa_evn -f $<

# cargo が更新の要否を判断するため、この入口は常に実行する。
.PHONY: firmware
firmware:
	cd firmware && cargo build --release

# ブートローダは ELF を読めないため、平坦なバイナリにしてからヘッダを付ける。
$(BUILD)/firmware.bin: firmware
	$(LLVM_OBJCOPY) -O binary $(FIRMWARE_ELF) $@

$(BUILD)/image_gen: $(NEORV32)/sw/image_gen/image_gen.c
	mkdir -p $(BUILD)
	$(CC) -o $@ $<

$(EXE): $(BUILD)/firmware.bin $(BUILD)/image_gen
	$(BUILD)/image_gen -i $< -o $@ -b 0x00000000 -t exe

# ファームウェアを UART から流し込んで実行する。FPGA のコンフィグとは経路が分かれる。
upload: $(EXE)
	uv run ../../tools/neorv32_upload/neorv32_upload.py --watch 5 $(PORT) $(EXE)

clean:
	rm -rf build

.PHONY: all sim load flash upload clean
