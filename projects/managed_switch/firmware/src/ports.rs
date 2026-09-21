//! スイッチのポートの並び。番号は hdl/managed_switch.vhd の PORT_* に合わせる。

pub const PORT_NAMES: [&str; 3] = ["rgmii", "rmii", "cpu"];
pub const PORT_COUNT: usize = PORT_NAMES.len();
pub const PORT_RGMII: usize = 0;
pub const PORT_RMII: usize = 1;
