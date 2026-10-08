//! 单击右 Shift 进出纯英文（`[shortcut] right_shift_english`）：进了之后按键一律交还应用，字母与标点原样输入。
//!
//! 修饰键单独按下抬起不是 KeyDown，要在 `recognizedEvents:` 里多要 FlagsChanged 才收得到；
//! 一旦不只要 KeyDown，IMK 就不再替输入法处理「点组句区外面就落定」，所以连 LeftMouseDown 一起要来自己落定。

use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags};

use super::{QingjianInputController, TextClient};
use crate::host;

/// 右 Shift 的键码（`kVK_RightShift`）。
const RIGHT_SHIFT: u16 = 60;

/// 要 IMK 送来的事件。不按配置变：IMK 什么时候来问、会不会缓存都不确定，开关在收到事件后再看。
pub(super) fn recognized_events() -> usize {
    (NSEventMask::KeyDown | NSEventMask::FlagsChanged | NSEventMask::LeftMouseDown).0 as usize
}

impl QingjianInputController {
    /// 修饰键变化：只有右 Shift 单独按下、中间没插进别的键又抬起才算一次单击。事件本身照常交给应用。
    ///
    /// 按下 / 抬起只看 Shift 标志：IMK 转来的事件不一定带区分左右的设备位。左 Shift 按着时右 Shift 抬起
    /// 标志不灭，不算单击。
    pub(super) fn note_flags_changed(&self, event: &NSEvent, client: TextClient<'_>) {
        let flags = event.modifierFlags();
        let right_shift = event.keyCode() == RIGHT_SHIFT;
        let down = flags.contains(NSEventModifierFlags::Shift);
        let others = flags.intersects(
            NSEventModifierFlags::Command
                | NSEventModifierFlags::Control
                | NSEventModifierFlags::Option,
        );
        let tapped = host::with(|h| {
            let tapped = right_shift && !down && h.right_shift_pending;
            h.right_shift_pending = h.right_shift_english && right_shift && down && !others;
            tapped
        })
        .unwrap_or(false);
        if tapped {
            self.toggle_plain_english(client);
        }
    }

    /// 进纯英文前把组着的拼音原样上屏、收起候选窗口；再单击一次回到正常输入。
    fn toggle_plain_english(&self, client: TextClient<'_>) {
        self.commit_raw(client);
        host::with(|h| {
            h.cancel_prediction();
            h.window.hide();
            let plain = !h.plain_english;
            h.set_plain_english(plain);
        });
    }

    /// 鼠标按下：顶替 IMK 缺省的鼠标处理，组着的拼音原样落定。
    pub(super) fn note_mouse_down(&self, client: TextClient<'_>) {
        if self.commit_raw(client) {
            host::with(|h| {
                h.cancel_prediction();
                h.window.hide();
            });
        }
    }
}
