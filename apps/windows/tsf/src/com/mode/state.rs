//! 当前中 / 英模式与语言栏更新回调，文本服务与语言栏按钮（[`super::ModeButton`]）共享。

use core::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::Win32::UI::TextServices::{ITfLangBarItemSink, TF_LBI_ICON, TF_LBI_STATUS};

use qingjian_platform::SwitchKeys;

use crate::com::key::TapKeys;

/// 当前中英模式 + 语言栏更新回调，文本服务与语言栏按钮共享（STA 单线程）。
pub(crate) struct ModeState {
    /// `true` 是英文模式。
    english: Cell<bool>,

    /// 英文模式是右 Shift 切出来的纯英文：键一律不吃，字母与标点由键盘布局原样交给应用。
    plain: Cell<bool>,

    /// 内置英文模式开关（`[general] english_mode`）：关掉后谁都不许切到英文。
    enabled: Cell<bool>,

    /// 勾着的中英切换键（`[shortcut] switch_mode`），单击判定与语言栏提示用。
    switch_keys: Cell<SwitchKeys>,

    /// 单击右 Shift 切纯英文（`[shortcut] right_shift_english`）。
    right_shift: Cell<bool>,

    /// 系统登记进来的语言栏更新回调；由 [`super::ModeButton`] 的 `ITfSource` 登记 / 撤销。
    pub(super) sink: RefCell<Option<ITfLangBarItemSink>>,
}

impl ModeState {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            english: Cell::new(false),
            plain: Cell::new(false),
            enabled: Cell::new(true),
            switch_keys: Cell::new(SwitchKeys::default()),
            right_shift: Cell::new(false),
            sink: RefCell::new(None),
        })
    }

    pub(crate) fn english(&self) -> bool {
        self.english.get()
    }

    pub(crate) fn plain(&self) -> bool {
        self.plain.get()
    }

    /// 切到普通的中 / 英模式（纯英文跟着清掉）。
    pub(crate) fn set_english(&self, english: bool) {
        self.set_mode(english, false);
    }

    /// `plain` 只在英文模式下算数。
    pub(crate) fn set_mode(&self, english: bool, plain: bool) {
        self.english.set(english);
        self.plain.set(english && plain);
    }

    /// 内置英文模式是否可用。
    pub(crate) fn enabled(&self) -> bool {
        self.enabled.get()
    }

    pub(crate) fn switch_keys(&self) -> SwitchKeys {
        self.switch_keys.get()
    }

    /// 单击判定认哪些键。
    pub(crate) fn tap_keys(&self) -> TapKeys {
        TapKeys {
            switch: self.switch_keys.get(),
            right_shift: self.right_shift.get(),
        }
    }

    /// 激活时按配置设一次。
    pub(crate) fn set_settings(&self, enabled: bool, switch_keys: SwitchKeys, right_shift: bool) {
        self.enabled.set(enabled);
        self.switch_keys.set(switch_keys);
        self.right_shift.set(right_shift);
    }

    /// 通知系统重取图标 / 文字。
    pub(crate) fn notify(&self) {
        if let Some(sink) = self.sink.borrow().as_ref() {
            let _ = unsafe { sink.OnUpdate(TF_LBI_ICON | TF_LBI_STATUS) };
        }
    }
}
