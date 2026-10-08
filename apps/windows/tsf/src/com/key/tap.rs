//! 单击中 / 英切换键的判定，喂的是击键 sink 的 `OnTestKeyDown` / `OnTestKeyUp`（被吃掉的键也经过它们，
//! 与 `WH_KEYBOARD` 钩子不同）。按下切换键到抬起之间没插进别的键，就是一次单击。
//!
//! 切换键来自 `[shortcut] switch_mode`，单击 Shift / 单击 Ctrl 可以都勾；Ctrl + Alt + Space 是组合键，走保留键。
//! `[shortcut] right_shift_english` 开着时右 Shift 单独判定（纯英文切换），`switch_mode` 的单击 Shift 只剩左 Shift。
//! 系统热键（如 Ctrl + Space）的第二个键被系统截走、到不了这里，看起来就像单击了 Ctrl：
//! 系统热键生效时调 [`KeyTap::cancel`] 作废这次按下。

use std::cell::Cell;

use windows::Win32::Foundation::LPARAM;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_LCONTROL, VK_LSHIFT, VK_RCONTROL, VK_RSHIFT, VK_SHIFT,
};

use qingjian_platform::{SwitchKey, SwitchKeys};

/// 右 Shift 的扫描码：TSF 送来的多半是不分左右的 `VK_SHIFT`，左右只能靠 lparam 里的扫描码分。
const RIGHT_SHIFT_SCAN: isize = 0x36;

/// 一次单击切的是什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tap {
    /// `switch_mode` 里勾着的单击键：中 / 英切换。
    Switch(SwitchKey),

    /// 右 Shift（`right_shift_english`）：纯英文切换。
    RightShift,
}

/// 哪些键算单击切换键：`switch_mode` 加右 Shift 开关。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TapKeys {
    pub(crate) switch: SwitchKeys,
    pub(crate) right_shift: bool,
}

#[derive(Default)]
pub(crate) struct KeyTap {
    /// 按下了哪个切换键、之后还没有别的键插进来。
    pressed: Cell<Option<Tap>>,
}

impl KeyTap {
    /// 任一键按下。`lparam` 第 30 位是按下前的状态（1 = 自动重复，不算新按下）。
    pub(crate) fn key_down(&self, vk: u32, lparam: LPARAM, keys: TapKeys) {
        let Some(key) = tap_key(keys, vk, lparam) else {
            self.cancel();
            return;
        };
        if (lparam.0 >> 30) & 1 != 0 {
            return;
        }
        // 两个切换键一起按（Ctrl + Shift 是系统换布局的键）不算单击
        let pressed = match self.pressed.get() {
            Some(other) if other != key => None,
            _ => Some(key),
        };
        self.pressed.set(pressed);
    }

    /// 任一键抬起；切换键单独抬起返回切的是什么，一次抬起只算一次。
    pub(crate) fn key_up(&self, vk: u32, lparam: LPARAM, keys: TapKeys) -> Option<Tap> {
        let key = tap_key(keys, vk, lparam)?;
        if self.pressed.get() == Some(key) {
            self.pressed.set(None);
            Some(key)
        } else {
            None
        }
    }

    /// 作废正按着的切换键（按下之后发生了别的事，这次抬起不算单击）。
    pub(crate) fn cancel(&self) {
        self.pressed.set(None);
    }
}

/// 这个键是不是单击切换键；组合键 Ctrl + Alt + Space 不走单击判定。
fn tap_key(keys: TapKeys, vk: u32, lparam: LPARAM) -> Option<Tap> {
    let is = |codes: [u16; 3]| codes.iter().any(|code| u32::from(*code) == vk);
    let shift = is([VK_SHIFT.0, VK_LSHIFT.0, VK_RSHIFT.0]);
    let right_shift = vk == u32::from(VK_RSHIFT.0)
        || (vk == u32::from(VK_SHIFT.0) && (lparam.0 >> 16) & 0xFF == RIGHT_SHIFT_SCAN);
    if shift && right_shift && keys.right_shift {
        Some(Tap::RightShift)
    } else if keys.switch.shift && shift {
        Some(Tap::Switch(SwitchKey::Shift))
    } else if keys.switch.control && is([VK_CONTROL.0, VK_LCONTROL.0, VK_RCONTROL.0]) {
        Some(Tap::Switch(SwitchKey::Control))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按下再抬起的 lparam（第 30 位为 0 表示新按下）。
    const DOWN: LPARAM = LPARAM(0);
    const VK_SHIFT_LEFT: u32 = 0xA0;
    const VK_SHIFT_GENERIC: u32 = 0x10;
    const VK_CONTROL_LEFT: u32 = 0xA2;
    const SHIFT: Option<Tap> = Some(Tap::Switch(SwitchKey::Shift));
    const CONTROL: Option<Tap> = Some(Tap::Switch(SwitchKey::Control));

    fn only(key: SwitchKey) -> TapKeys {
        TapKeys {
            switch: SwitchKeys::NONE.with(key, true),
            right_shift: false,
        }
    }

    /// 扫描码放在 lparam 的 16–23 位。
    fn scan(code: isize) -> LPARAM {
        LPARAM(code << 16)
    }

    #[test]
    fn shift_tap_fires_only_when_nothing_else_interrupts() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), SHIFT);
        // 一次抬起只算一次
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), None);

        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        tap.key_down(0x41, DOWN, keys); // 中间插了一个 A
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), None);
    }

    #[test]
    fn only_checked_keys_fire() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Control);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), None);
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        assert_eq!(tap.key_up(VK_CONTROL_LEFT, DOWN, keys), CONTROL);

        // 一个都不勾、只勾组合键：修饰键单击都不算
        let none = TapKeys {
            switch: SwitchKeys::NONE,
            right_shift: false,
        };
        for keys in [none, only(SwitchKey::CtrlAltSpace)] {
            tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
            assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), None);
            tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
            assert_eq!(tap.key_up(VK_CONTROL_LEFT, DOWN, keys), None);
        }
    }

    #[test]
    fn both_taps_work_when_both_are_checked_but_not_together() {
        let tap = KeyTap::default();
        let mut keys = only(SwitchKey::Shift);
        keys.switch = keys.switch.with(SwitchKey::Control, true);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), SHIFT);
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        assert_eq!(tap.key_up(VK_CONTROL_LEFT, DOWN, keys), CONTROL);

        // Ctrl + Shift 一起按：谁抬起都不算
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), None);
        assert_eq!(tap.key_up(VK_CONTROL_LEFT, DOWN, keys), None);
    }

    #[test]
    fn a_system_hotkey_cancels_the_pending_tap() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Control);
        // Ctrl + Space 的 Space 被系统截走，这里只看到 Ctrl 按下又抬起
        tap.key_down(VK_CONTROL_LEFT, DOWN, keys);
        tap.cancel();
        assert_eq!(tap.key_up(VK_CONTROL_LEFT, DOWN, keys), None);
    }

    #[test]
    fn auto_repeat_does_not_rearm() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift);
        // 第 30 位为 1：自动重复，不算新按下
        let repeat = LPARAM(1 << 30);
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), SHIFT);
        tap.key_down(VK_SHIFT_LEFT, repeat, keys);
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), None);
    }

    #[test]
    fn right_shift_is_told_apart_by_its_scan_code() {
        let tap = KeyTap::default();
        let keys = TapKeys {
            right_shift: true,
            ..only(SwitchKey::Shift)
        };
        // 不分左右的 VK_SHIFT：扫描码 0x36 是右 Shift，0x2A 是左 Shift
        tap.key_down(VK_SHIFT_GENERIC, scan(RIGHT_SHIFT_SCAN), keys);
        assert_eq!(
            tap.key_up(VK_SHIFT_GENERIC, scan(RIGHT_SHIFT_SCAN), keys),
            Some(Tap::RightShift)
        );
        tap.key_down(VK_SHIFT_GENERIC, scan(0x2A), keys);
        assert_eq!(tap.key_up(VK_SHIFT_GENERIC, scan(0x2A), keys), SHIFT);
        // 分左右的 VK_RSHIFT 直接认
        tap.key_down(u32::from(VK_RSHIFT.0), DOWN, keys);
        assert_eq!(
            tap.key_up(u32::from(VK_RSHIFT.0), DOWN, keys),
            Some(Tap::RightShift)
        );
        // 两个 Shift 一起按：都不算
        tap.key_down(VK_SHIFT_GENERIC, scan(0x2A), keys);
        tap.key_down(VK_SHIFT_GENERIC, scan(RIGHT_SHIFT_SCAN), keys);
        assert_eq!(
            tap.key_up(VK_SHIFT_GENERIC, scan(RIGHT_SHIFT_SCAN), keys),
            None
        );
    }

    #[test]
    fn right_shift_falls_back_to_the_switch_key_when_turned_off() {
        let tap = KeyTap::default();
        let keys = only(SwitchKey::Shift);
        tap.key_down(VK_SHIFT_GENERIC, scan(RIGHT_SHIFT_SCAN), keys);
        assert_eq!(
            tap.key_up(VK_SHIFT_GENERIC, scan(RIGHT_SHIFT_SCAN), keys),
            SHIFT
        );
        // 只开右 Shift、不勾单击 Shift：左 Shift 不算，右 Shift 照样算
        let keys = TapKeys {
            switch: SwitchKeys::NONE,
            right_shift: true,
        };
        tap.key_down(VK_SHIFT_LEFT, DOWN, keys);
        assert_eq!(tap.key_up(VK_SHIFT_LEFT, DOWN, keys), None);
        tap.key_down(VK_SHIFT_GENERIC, scan(RIGHT_SHIFT_SCAN), keys);
        assert_eq!(
            tap.key_up(VK_SHIFT_GENERIC, scan(RIGHT_SHIFT_SCAN), keys),
            Some(Tap::RightShift)
        );
    }
}
