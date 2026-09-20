DEVICE := LFD2NX-40-8BG256C
BUILD  := build

VHDL := led_sequence.vhd

GHDL_FLAGS := --workdir=$(BUILD)

all: $(BUILD)/led_sequence.bit

# ghdl -m で解析結果をライブラリに保存すると、解析時刻で依存の新旧を判定するため、システム時刻が巻き戻ると失敗する。
# GHDL には解析順だけを出させ、解析から合成までを yosys の 1 プロセスで行う。
$(BUILD)/led_sequence.json: $(VHDL) Makefile
	mkdir -p $(BUILD)
	ghdl -i $(GHDL_FLAGS) $(VHDL)
	yosys -m ghdl -l $(BUILD)/yosys.log -q -p \
		"ghdl $$(ghdl --elab-order $(GHDL_FLAGS) led_sequence 2>/dev/null | tr '\n' ' ') -e led_sequence; \
		 synth_nexus -top led_sequence -json $@"

# PDC の create_clock は入力バッファより先のクロック網に伝わらないため、目標の周波数は nextpnr-nexus に渡す。
$(BUILD)/led_sequence.fasm: $(BUILD)/led_sequence.json led_sequence.pdc
	nextpnr-nexus --device $(DEVICE) --pdc led_sequence.pdc --freq 25 --json $< \
		--fasm $@ --report $(BUILD)/report.json -l $(BUILD)/nextpnr.log -q

$(BUILD)/led_sequence.bit: $(BUILD)/led_sequence.fasm
	prjoxide pack $< $@

SIM_VHDL := $(VHDL) led_sequence_tb.vhd

# ghdl -c は解析結果をライブラリに保存しないため、システム時刻が巻き戻っても失敗しない。
sim: $(SIM_VHDL)
	mkdir -p build/sim
	ghdl -c --workdir=build/sim -o build/sim/led_sequence_tb $(SIM_VHDL) -e led_sequence_tb
	build/sim/led_sequence_tb --assert-level=error

# SRAM に書き込む。電源を切ると消える。
load: $(BUILD)/led_sequence.bit
	openFPGALoader -b certusnx_versa_evn $<

# SPI Flash に書き込む。電源を入れ直しても残る。
flash: $(BUILD)/led_sequence.bit
	openFPGALoader -b certusnx_versa_evn -f $<

clean:
	rm -rf build

.PHONY: all sim load flash clean
