//! 悬浮状态条随模式、双拼方案与开关变化。

use crate::support::*;

#[test]
fn status_bar_mode_click_changes_the_global_mode() {
    let config = RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
        plain: false,
    });

    // 点「中」：状态条翻成「英」，之后每个 DLL 来取都拿到英文（全局一份，不是取一次就清）。
    router.handle_status_event(StatusEvent::ToggleMode);
    assert_eq!(recorder.calls().last(), Some(&Some("英".to_owned())));
    assert_eq!(synced_mode(&mut router, SESSION), Some(true));
    assert_eq!(synced_mode(&mut router, SESSION), Some(true));
    assert_eq!(synced_mode(&mut router, SessionId(2)), Some(true));
}

#[test]
fn status_bar_mode_click_is_ignored_when_builtin_english_is_off() {
    let config = RouterConfig {
        status_enabled: true,
        english_mode: false,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
        plain: false,
    });
    assert_eq!(recorder.calls().last(), Some(&Some("中".to_owned())));

    // 关掉内置英文模式：点「中」不翻成「英」，DLL 取到的也是中文（DLL 那边同样会拦）
    router.handle_status_event(StatusEvent::ToggleMode);
    assert_eq!(recorder.calls().last(), Some(&Some("中".to_owned())));
    assert_eq!(synced_mode(&mut router, SESSION), Some(false));
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
        plain: false,
    });
    assert_eq!(synced_mode(&mut router, SESSION), Some(false));
}

#[test]
fn status_bar_follows_mode_when_enabled() {
    let config = RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    // 中文 → 英文：各刷一次；会话关掉（应用退出）不收；切成别的输入法才收起。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
        plain: false,
    });
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
        plain: false,
    });
    router.handle(ClientMessage::CloseSession { session: SESSION });
    assert_eq!(
        recorder.calls(),
        vec![Some("中".to_owned()), Some("英".to_owned())]
    );

    router.handle(ClientMessage::ImeSwitched { session: SESSION });
    assert_eq!(recorder.calls().last(), Some(&None));
}

#[test]
fn status_bar_shows_shuangpin_scheme_in_chinese() {
    let config = RouterConfig {
        status_enabled: true,
        scheme: Scheme::Shuangpin(ShuangpinScheme::Xiaohe),
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
        plain: false,
    });

    assert_eq!(recorder.calls(), vec![Some("中 · 小鹤双拼".to_owned())]);
}

#[test]
fn status_bar_stays_hidden_when_disabled() {
    let mut router = router();
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
        plain: false,
    });

    assert_eq!(recorder.calls(), vec![None]);
}

#[test]
fn indicator_menu_toggles_status_bar() {
    use qingjian_platform::protocol::IndicatorCommand;

    let mut router = router();
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
        plain: false,
    });

    // 任务栏图标菜单里点「悬浮状态条」：关着的打开，再点收起。
    let toggle = ClientMessage::Indicator {
        session: SESSION,
        command: IndicatorCommand::ToggleStatusBar,
    };
    router.handle(toggle.clone());
    assert_eq!(recorder.calls().last(), Some(&Some("中".to_owned())));
    router.handle(toggle);
    assert_eq!(recorder.calls().last(), Some(&None));
}

#[test]
fn mode_is_shared_by_every_app() {
    let config = RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));
    let other_app = SessionId(2);

    // 一个应用里切到英文：别的应用、之后新开的应用来取都是英文。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
        plain: false,
    });
    assert_eq!(synced_mode(&mut router, other_app), Some(true));
    assert_eq!(synced_mode(&mut router, SessionId(3)), Some(true));

    // 切成别的输入法收起状态条；再有应用来取模式（又切回青简）就重新显示，模式照旧。
    router.handle(ClientMessage::ImeSwitched { session: SESSION });
    assert_eq!(recorder.calls().last(), Some(&None));
    assert_eq!(synced_mode(&mut router, other_app), Some(true));
    assert_eq!(recorder.calls().last(), Some(&Some("英".to_owned())));
}

#[test]
fn plain_english_is_shared_and_cleared_by_any_other_switch() {
    let mut router = router_with(RouterConfig::default());
    let other_app = SessionId(2);
    // 右 Shift 切出来的纯英文：别的应用来取也是纯英文
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
        plain: true,
    });
    assert_eq!(synced_plain(&mut router, other_app), (Some(true), true));
    // 状态条切模式：回中文，纯英文跟着清掉
    router.handle_status_event(StatusEvent::ToggleMode);
    assert_eq!(synced_plain(&mut router, other_app), (Some(false), false));
    // 中文模式下报来的 plain 不算数
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
        plain: true,
    });
    assert_eq!(synced_plain(&mut router, other_app), (Some(false), false));
    // 配置关掉右 Shift 纯英文：报来也不收
    let mut router = router_with(RouterConfig {
        right_shift_english: false,
        ..RouterConfig::default()
    });
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
        plain: true,
    });
    assert_eq!(synced_plain(&mut router, other_app), (Some(true), false));
}

/// 会话取一次 `SyncMode`，返回全局模式与纯英文标记。
fn synced_plain(router: &mut Router, session: SessionId) -> (Option<bool>, bool) {
    match router.handle(ClientMessage::SyncMode { session }) {
        Some(ServerMessage::ModeSync { english, plain, .. }) => (english, plain),
        other => panic!("SyncMode 应回 ModeSync，实际 {other:?}"),
    }
}

/// 会话取一次 `SyncMode`，返回它拿到的全局模式。
fn synced_mode(router: &mut Router, session: SessionId) -> Option<bool> {
    match router.handle(ClientMessage::SyncMode { session }) {
        Some(ServerMessage::ModeSync { english, .. }) => english,
        other => panic!("SyncMode 应回 ModeSync，实际 {other:?}"),
    }
}
