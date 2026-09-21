//! スイッチの MAC アドレステーブル。

use super::Switch;
use satcat5_pac::switch_core::query_ctrl;

/// 空の項目は、MAC アドレスが全て 1 で返る。
const MAC_EMPTY: [u8; 6] = [0xFF; 6];

impl Switch {
    /// MAC アドレステーブルは、QUERY_CTRL に操作を書いて読み書きする。
    /// 操作の間は QUERY_CTRL を読むと操作が返り、終わると IDLE になって、読み出しならポートの番号が返る。
    fn query(&self, operation: impl FnOnce(&mut query_ctrl::W) -> &mut query_ctrl::W) -> u16 {
        let ctrl = self.core().query_ctrl();
        while !ctrl.read().opcode().is_idle() {}
        ctrl.write(operation);
        loop {
            let status = ctrl.read();
            if status.opcode().is_idle() {
                return status.arg().bits();
            }
        }
    }

    /// 項目が空なら None を返す。MAC アドレスは、上位 2 バイトが QUERY_MAC_MSB に、下位 4 バイトが QUERY_MAC_LSB に入る。
    pub fn mac_entry(&self, index: u16) -> Option<([u8; 6], usize)> {
        let port = self.query(|w| w.opcode().read().arg().set(index));
        let core = self.core();
        let msb = core.query_mac_msb().read().mac().bits();
        let lsb = core.query_mac_lsb().read().bits();
        let mut mac = [0; 6];
        mac[..2].copy_from_slice(&msb.to_be_bytes());
        mac[2..].copy_from_slice(&lsb.to_be_bytes());
        (mac != MAC_EMPTY).then_some((mac, port.into()))
    }

    pub fn mac_clear(&self) {
        self.query(|w| w.opcode().clear());
    }
}
