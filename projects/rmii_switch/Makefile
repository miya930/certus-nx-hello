PORT_COUNT ?= 4

DEVICE  := LFD2NX-40-8BG256C
SATCAT5 := ../../third_party/satcat5/src/vhdl
NEXUS   := ../../third_party/satcat5_nexus
PATCHES := $(NEXUS)/patches
BUILD   := build/ports$(PORT_COUNT)

# GHDL で合成できない SatCat5 のファイルは、パッチを当てたコピーを元のファイルの代わりに使う。
PATCHED := $(patsubst $(PATCHES)/%.patch,build/patched/%,$(wildcard $(PATCHES)/*.patch))

# SatCat5 の Lattice 向けの同期回路は、SB_DFFR 以外に iCE40 固有の記述がないため、そのまま使う。
VHDL := $(filter-out $(addprefix $(SATCAT5)/common/,$(notdir $(PATCHED))),$(wildcard $(SATCAT5)/common/*.vhd)) \
        $(PATCHED) \
        $(SATCAT5)/lattice/ice40_sync.vhd \
        $(wildcard $(NEXUS)/primitives/*.vhd) \
        rmii_switch.vhd

# Nexus のプリミティブは VHDL の実体を持たないため、未結合の警告にとどめてブラックボックスとして yosys に渡す。
GHDL_FLAGS := --workdir=$(BUILD) --warn-no-binding

all: $(BUILD)/report.json

build/patched/%: $(SATCAT5)/common/% $(PATCHES)/%.patch
	mkdir -p $(dir $@)
	patch -s -o $@ $< $(PATCHES)/$*.patch

# ghdl -m で解析結果をライブラリに保存すると、解析時刻で依存の新旧を判定するため、システム時刻が巻き戻ると失敗する。
# GHDL には解析順だけを出させ、解析から合成までを yosys の 1 プロセスで行う。
# VHDL の assert 文は合成後もセルとして残り、nextpnr-nexus が扱えないため、合成の前に取り除く。
$(BUILD)/rmii_switch.json: $(VHDL) Makefile
	mkdir -p $(BUILD)
	ghdl -i $(GHDL_FLAGS) $(VHDL)
	yosys -m ghdl -l $(BUILD)/yosys.log -q -p \
		"ghdl --warn-no-binding -gPORT_COUNT=$(PORT_COUNT) $$(ghdl --elab-order $(GHDL_FLAGS) rmii_switch 2>/dev/null | tr '\n' ' ') -e rmii_switch; \
		 chformal -remove; \
		 synth_nexus -top rmii_switch -json $@"

# PDC の create_clock は入力バッファより先のクロック網に伝わらないため、
# 全クロックの目標を REF_CLK の 50 MHz にそろえる。25 MHz の SYSTEM_25M_CLK は実際より厳しい条件で確認することになる。
$(BUILD)/report.json: $(BUILD)/rmii_switch.json rmii_switch.pdc
	nextpnr-nexus --device $(DEVICE) --pdc rmii_switch.pdc --freq 50 --json $< --report $@ -l $(BUILD)/nextpnr.log -q

# シミュレーションでは、Nexus のプリミティブを satcat5_nexus の sim のモデルで置き換える。
# ghdl -c は解析結果をライブラリに保存しないため、システム時刻が巻き戻っても失敗しない。
SIM_VHDL := $(VHDL) $(wildcard $(NEXUS)/sim/*.vhd) rmii_switch_tb.vhd

sim: $(SIM_VHDL)
	mkdir -p build/sim
	ghdl -c --workdir=build/sim -o build/sim/rmii_switch_tb $(SIM_VHDL) -e rmii_switch_tb
	build/sim/rmii_switch_tb --assert-level=error --ieee-asserts=disable

clean:
	rm -rf build

.PHONY: all sim clean
