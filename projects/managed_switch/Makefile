DEVICE  := LFD2NX-40-8BG256C
SATCAT5 := ../../third_party/satcat5/src/vhdl
NEORV32 := ../../third_party/neorv32
NEXUS   := ../../third_party/satcat5_nexus
PATCHES := $(NEXUS)/patches
BUILD   := build
PORT    ?= /dev/ttyUSB1

LLVM_OBJCOPY := $(shell find $$(rustc --print sysroot) -name llvm-objcopy | head -1)
FIRMWARE_ELF := $(BUILD)/firmware/riscv32imc-unknown-none-elf/release/managed_switch_firmware
EXE          := $(BUILD)/neorv32_exe.bin

# GHDL で合成できない SatCat5 のファイルは、パッチを当てたコピーを元のファイルの代わりに使う。
PATCHED := $(patsubst $(PATCHES)/%.patch,build/patched/%,$(wildcard $(PATCHES)/*.patch))

# NEORV32 は VHDL-2008 で書かれ、自身を neorv32 ライブラリに置くことを前提にしている。
# SatCat5 も同じライブラリに入れると、両者の work 参照が同じ場所を指す。
# Nexus のプリミティブは VHDL の実体を持たず、合成ではブラックボックスになるため、結び付かない警告は止める。
GHDL_FLAGS := --std=08 --work=neorv32 --workdir=$(BUILD) --warn-no-binding

# SatCat5 は、PTP や VLAN などの使わない機能の信号も宣言し、機能を有効にしたときだけ値を入れる。
# この設計ではそれらの機能を切っているため、合成で値が入らない信号の警告が SatCat5 のファイルから出る。
# 警告は全て SatCat5 の使わない機能のものなので、合成では止める。
# GHDL が RAM と ROM を見つけるたびに出す知らせも、警告ではないため止める。
# NEORV32 が assert 文で出す構成の知らせは、GHDL の選択肢では止められないため、端末に残る。
SYNTH_FLAGS := --std=08 --work=neorv32 --warn-no-binding -Wno-nowrite --no-notes

# NEORV32 は解析順を file_list_core.f に持っている。
CORE_VHDL := $(shell sed 's|$$NEORV32_HOME|$(NEORV32)|g' $(NEORV32)/rtl/file_list_core.f)

# SatCat5 の Lattice 向けの同期回路は、SB_DFFR 以外に iCE40 固有の記述がないため、そのまま使う。
VHDL := $(CORE_VHDL) \
        $(filter-out $(addprefix $(SATCAT5)/common/,$(notdir $(PATCHED))),$(wildcard $(SATCAT5)/common/*.vhd)) \
        $(PATCHED) \
        $(SATCAT5)/lattice/ice40_sync.vhd \
        $(wildcard $(NEXUS)/primitives/*.vhd) \
        hdl/managed_switch.vhd

all: $(BUILD)/managed_switch.bit

build/patched/%: $(SATCAT5)/common/% $(PATCHES)/%.patch
	mkdir -p $(dir $@)
	patch -s -o $@ $< $(PATCHES)/$*.patch

# ghdl -m で解析結果をライブラリに保存すると、解析時刻で依存の新旧を判定するため、システム時刻が巻き戻ると失敗する。
# GHDL には解析順だけを出させ、解析から合成までを yosys の 1 プロセスで行う。
# VHDL の assert 文は合成後もセルとして残り、nextpnr-nexus が扱えないため、合成の前に取り除く。
$(BUILD)/managed_switch.json: $(VHDL) Makefile
	mkdir -p $(BUILD)
	ghdl -i $(GHDL_FLAGS) $(VHDL)
	yosys -m ghdl -l $(BUILD)/yosys.log -q -p \
		"ghdl $(SYNTH_FLAGS) $$(ghdl --elab-order $(GHDL_FLAGS) managed_switch 2>/dev/null | tr '\n' ' ') -e managed_switch; \
		 chformal -remove; \
		 synth_nexus -top managed_switch -json $@"

# --freq は設計全体に 1 つの目標しか与えられず、PDC の create_clock も --sdc も内部のクロック網に届かない。
# この設計は RGMII の 125 MHz、RMII の 50 MHz、スイッチコアと CPU の 25 MHz を持つため、目標を一番高い 125 MHz に合わせる。
# 遅いクロックの回路まで 125 MHz で評価されるので失敗を許し、クロックごとの要求は check_timing.py で確かめる。
$(BUILD)/managed_switch.fasm: $(BUILD)/managed_switch.json hdl/managed_switch.pdc tools/check_timing.py
	nextpnr-nexus --device $(DEVICE) --pdc hdl/managed_switch.pdc --freq 125 --timing-allow-fail --json $< \
		--fasm $@ --report $(BUILD)/report.json -l $(BUILD)/nextpnr.log -q
	python3 tools/check_timing.py $(BUILD)/report.json

$(BUILD)/managed_switch.bit: $(BUILD)/managed_switch.fasm
	prjoxide pack $< $@

SIM_VHDL := $(VHDL) $(wildcard $(NEXUS)/sim/*.vhd) hdl/managed_switch_tb.vhd

# シミュレーションでは、Nexus のプリミティブを satcat5_nexus の sim のモデルで置き換える。
# ghdl -c は解析結果をライブラリに保存しないため、システム時刻が巻き戻っても失敗しない。
# コアのメモリは初期値を持たないまま読み出されることがあり、
# IEEE のライブラリが metavalue の警告を出すため、その警告は止める。
sim: $(SIM_VHDL)
	mkdir -p build/sim
	ghdl -c --std=08 --work=neorv32 --workdir=build/sim -o build/sim/managed_switch_tb $(SIM_VHDL) -e managed_switch_tb
	build/sim/managed_switch_tb --assert-level=error --ieee-asserts=disable

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

# SRAM に書き込む。電源を切ると消える。
load: $(BUILD)/managed_switch.bit
	openFPGALoader -b certusnx_versa_evn $<

# SPI Flash に書き込む。電源を入れ直しても残る。
flash: $(BUILD)/managed_switch.bit
	openFPGALoader -b certusnx_versa_evn -f $<

clean:
	rm -rf build

.PHONY: all sim load flash upload clean
