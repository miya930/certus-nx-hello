DEVICE  := LFD2NX-40-8BG256C
SATCAT5 := ../../third_party/satcat5/src/vhdl
NEORV32 := ../../third_party/neorv32
NEXUS   := ../../third_party/satcat5_nexus
PATCHES := $(NEXUS)/patches
BUILD   := build
PORT    ?= /dev/ttyUSB1

LLVM_OBJCOPY := $(shell find $$(rustc --print sysroot) -name llvm-objcopy | head -1)
FIRMWARE_ELF := $(BUILD)/firmware/riscv32imc-unknown-none-elf/release/ip_endpoint_firmware
EXE          := $(BUILD)/neorv32_exe.bin

# GHDL で合成できない SatCat5 のファイルは、パッチを当てたコピーを元のファイルの代わりに使う。
PATCHED := $(patsubst $(PATCHES)/%.patch,build/patched/%,$(wildcard $(PATCHES)/*.patch))

# NEORV32 は VHDL-2008 で書かれ、自身を neorv32 ライブラリに置くことを前提にしている。
# SatCat5 も同じライブラリに入れると、両者の work 参照が同じ場所を指す。
GHDL_FLAGS := --std=08 --work=neorv32 --workdir=$(BUILD) --warn-no-binding

# NEORV32 は解析順を file_list_core.f に持っている。
CORE_VHDL := $(shell sed 's|$$NEORV32_HOME|$(NEORV32)|g' $(NEORV32)/rtl/file_list_core.f)

# SatCat5 の Lattice 向けの同期回路は、SB_DFFR 以外に iCE40 固有の記述がないため、そのまま使う。
VHDL := $(CORE_VHDL) \
        $(filter-out $(addprefix $(SATCAT5)/common/,$(notdir $(PATCHED))),$(wildcard $(SATCAT5)/common/*.vhd)) \
        $(PATCHED) \
        $(SATCAT5)/lattice/ice40_sync.vhd \
        $(wildcard $(NEXUS)/primitives/*.vhd) \
        ip_endpoint.vhd

all: $(BUILD)/ip_endpoint.bit

build/patched/%: $(SATCAT5)/common/% $(PATCHES)/%.patch
	mkdir -p $(dir $@)
	patch -s -o $@ $< $(PATCHES)/$*.patch

# ghdl -m で解析結果をライブラリに保存すると、解析時刻で依存の新旧を判定するため、システム時刻が巻き戻ると失敗する。
# GHDL には解析順だけを出させ、解析から合成までを yosys の 1 プロセスで行う。
# VHDL の assert 文は合成後もセルとして残り、nextpnr-nexus が扱えないため、合成の前に取り除く。
$(BUILD)/ip_endpoint.json: $(VHDL) Makefile
	mkdir -p $(BUILD)
	ghdl -i $(GHDL_FLAGS) $(VHDL)
	yosys -m ghdl -l $(BUILD)/yosys.log -q -p \
		"ghdl --std=08 --work=neorv32 --warn-no-binding $$(ghdl --elab-order $(GHDL_FLAGS) ip_endpoint 2>/dev/null | tr '\n' ' ') -e ip_endpoint; \
		 chformal -remove; \
		 synth_nexus -top ip_endpoint -json $@"

# --freq は設計全体に 1 つの目標しか与えられず、PDC の create_clock も --sdc も内部のクロック網に届かない。
# この設計は RGMII の 125 MHz と、スイッチコアと CPU の 25 MHz を持つため、目標を高いほうに合わせる。
# 25 MHz の回路まで 125 MHz で評価されるので失敗を許し、クロックごとの要求は check_timing.py で確かめる。
$(BUILD)/ip_endpoint.fasm: $(BUILD)/ip_endpoint.json ip_endpoint.pdc check_timing.py
	nextpnr-nexus --device $(DEVICE) --pdc ip_endpoint.pdc --freq 125 --timing-allow-fail --json $< \
		--fasm $@ --report $(BUILD)/report.json -l $(BUILD)/nextpnr.log -q
	python3 check_timing.py $(BUILD)/report.json

$(BUILD)/ip_endpoint.bit: $(BUILD)/ip_endpoint.fasm
	prjoxide pack $< $@

SIM_VHDL := $(VHDL) $(wildcard $(NEXUS)/sim/*.vhd) ip_endpoint_tb.vhd

# シミュレーションでは、Nexus のプリミティブを satcat5_nexus の sim のモデルで置き換える。
# ghdl -c は解析結果をライブラリに保存しないため、システム時刻が巻き戻っても失敗しない。
# コアのメモリは初期値を持たないまま読み出されることがあり、
# IEEE のライブラリが metavalue の警告を出すため、その警告は止める。
sim: $(SIM_VHDL)
	mkdir -p build/sim
	ghdl -c --std=08 --work=neorv32 --workdir=build/sim -o build/sim/ip_endpoint_tb $(SIM_VHDL) -e ip_endpoint_tb
	build/sim/ip_endpoint_tb --assert-level=error --ieee-asserts=disable

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
load: $(BUILD)/ip_endpoint.bit
	openFPGALoader -b certusnx_versa_evn $<

# SPI Flash に書き込む。電源を入れ直しても残る。
flash: $(BUILD)/ip_endpoint.bit
	openFPGALoader -b certusnx_versa_evn -f $<

clean:
	rm -rf build

.PHONY: all sim load flash upload clean
