DEVICE  := LFD2NX-40-8BG256C
SATCAT5 := ../../third_party/satcat5/src/vhdl
NEXUS   := ../../third_party/satcat5_nexus
PATCHES := $(NEXUS)/patches
BUILD   := build

# GHDL で合成できない SatCat5 のファイルは、パッチを当てたコピーを元のファイルの代わりに使う。
PATCHED := $(patsubst $(PATCHES)/%.patch,build/patched/%,$(wildcard $(PATCHES)/*.patch))

# SatCat5 の Lattice 向けの同期回路は、SB_DFFR 以外に iCE40 固有の記述がないため、そのまま使う。
VHDL := $(filter-out $(addprefix $(SATCAT5)/common/,$(notdir $(PATCHED))),$(wildcard $(SATCAT5)/common/*.vhd)) \
        $(PATCHED) \
        $(SATCAT5)/lattice/ice40_sync.vhd \
        $(wildcard $(NEXUS)/primitives/*.vhd) \
        rgmii_phy_test.vhd

# Nexus のプリミティブは VHDL の実体を持たないため、未結合の警告にとどめてブラックボックスとして yosys に渡す。
GHDL_FLAGS := --workdir=$(BUILD) --warn-no-binding

all: $(BUILD)/rgmii_phy_test.bit

build/patched/%: $(SATCAT5)/common/% $(PATCHES)/%.patch
	mkdir -p $(dir $@)
	patch -s -o $@ $< $(PATCHES)/$*.patch

# ghdl -m で解析結果をライブラリに保存すると、解析時刻で依存の新旧を判定するため、システム時刻が巻き戻ると失敗する。
# GHDL には解析順だけを出させ、解析から合成までを yosys の 1 プロセスで行う。
# VHDL の assert 文は合成後もセルとして残り、nextpnr-nexus が扱えないため、合成の前に取り除く。
$(BUILD)/rgmii_phy_test.json: $(VHDL) Makefile
	mkdir -p $(BUILD)
	ghdl -i $(GHDL_FLAGS) $(VHDL)
	yosys -m ghdl -l $(BUILD)/yosys.log -q -p \
		"ghdl --warn-no-binding $$(ghdl --elab-order $(GHDL_FLAGS) rgmii_phy_test 2>/dev/null | tr '\n' ' ') -e rgmii_phy_test; \
		 chformal -remove; \
		 synth_nexus -top rgmii_phy_test -json $@"

# PDC の create_clock は入力バッファより先のクロック網に伝わらないため、目標の周波数は nextpnr-nexus に渡す。
$(BUILD)/rgmii_phy_test.fasm: $(BUILD)/rgmii_phy_test.json rgmii_phy_test.pdc
	nextpnr-nexus --device $(DEVICE) --pdc rgmii_phy_test.pdc --freq 125 --json $< \
		--fasm $@ --report $(BUILD)/report.json -l $(BUILD)/nextpnr.log -q

$(BUILD)/rgmii_phy_test.bit: $(BUILD)/rgmii_phy_test.fasm
	prjoxide pack $< $@

# シミュレーションでは、Nexus のプリミティブを satcat5_nexus の sim のモデルで置き換える。
# ghdl -c は解析結果をライブラリに保存しないため、システム時刻が巻き戻っても失敗しない。
SIM_VHDL := $(VHDL) $(wildcard $(NEXUS)/sim/*.vhd) rgmii_phy_test_tb.vhd

sim: $(SIM_VHDL)
	mkdir -p build/sim
	ghdl -c --workdir=build/sim -o build/sim/rgmii_phy_test_tb $(SIM_VHDL) -e rgmii_phy_test_tb
	build/sim/rgmii_phy_test_tb --assert-level=error --ieee-asserts=disable

# SRAM に書き込む。電源を切ると消える。
load: $(BUILD)/rgmii_phy_test.bit
	openFPGALoader -b certusnx_versa_evn $<

# SPI Flash に書き込む。電源を入れ直しても残る。
flash: $(BUILD)/rgmii_phy_test.bit
	openFPGALoader -b certusnx_versa_evn -f $<

clean:
	rm -rf build

.PHONY: all sim load flash clean
