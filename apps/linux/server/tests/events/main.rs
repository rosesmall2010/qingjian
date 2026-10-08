//! 能力优先级、焦点边界和薄插件事件的业务决策。
mod support;
use self::support::{caps, compose, event, key, router, router_with};
use qingjian_linux_server::RouterConfig;
use serde_json::json;

#[test]
fn capability_priority_discards_same_privacy_transitions_and_never_commits_old_text() {
    let mut router = router();
    assert_eq!(compose(&mut router, 1, "nihao")["outcome"], "Consumed");
    caps(&mut router, 1, true, false, false);
    assert!(router.is_private());
    assert_eq!(key(&mut router, 1, 13, None, false)["commit"], json!(null));
    compose(&mut router, 1, "nihao");
    caps(&mut router, 1, true, true, false);
    assert_eq!(
        key(&mut router, 1, 32, Some(' '), false)["outcome"],
        "Passthrough"
    );
    assert_eq!(
        event(
            &mut router,
            1,
            json!({"Deactivate": {"focus_out": false, "client_preedit": false, "capability_changed": false}})
        )["commit"],
        json!(null)
    );
    caps(&mut router, 1, false, false, true);
    assert_eq!(compose(&mut router, 1, "ni")["outcome"], "Passthrough");
    caps(&mut router, 1, false, false, false);
    assert_eq!(compose(&mut router, 1, "nihao")["outcome"], "Consumed");
    assert!(!router.is_private());
    assert_eq!(key(&mut router, 1, 32, Some(' '), false)["commit"], "你好");
}
#[test]
fn shift_click_and_deactivate_decisions_live_in_server() {
    let mut router = router();
    compose(&mut router, 1, "ni");
    assert_eq!(
        key(&mut router, 1, 16, None, false)["outcome"],
        "Passthrough"
    );
    assert_eq!(key(&mut router, 1, 16, None, true)["commit"], "ni");
    assert_eq!(key(&mut router, 1, 16, None, true)["commit"], json!(null));
    key(&mut router, 1, 16, None, false);
    key(&mut router, 1, 16, None, true);
    compose(&mut router, 1, "ni");
    let response = key(&mut router, 1, 9, None, false);
    let identity = response["identity"].clone();
    let expected = response["frame"]["candidates"]["items"][0]["text"].clone();
    assert_eq!(
        event(
            &mut router,
            1,
            json!({"Candidate": {"identity": identity, "index": 0}})
        )["commit"],
        expected
    );
    assert_eq!(
        event(
            &mut router,
            1,
            json!({"Candidate": {"identity": identity, "index": 0}})
        )["commit"],
        json!(null)
    );
    for (client_preedit, capability_changed, expected) in [
        (true, false, json!(null)),
        (false, false, json!("ni")),
        (false, true, json!(null)),
    ] {
        compose(&mut router, 1, "ni");
        assert_eq!(
            event(
                &mut router,
                1,
                json!({"Deactivate": {"focus_out": true, "client_preedit": client_preedit, "capability_changed": capability_changed}})
            )["commit"],
            expected
        );
    }
}
#[test]
fn right_shift_click_toggles_plain_english() {
    let mut router = router();
    compose(&mut router, 1, "ni");
    // 单击右 Shift：组着的拼音原样上屏，之后字母、标点都放行
    key(&mut router, 1, 0xA1, None, false);
    assert_eq!(key(&mut router, 1, 0xA1, None, true)["commit"], "ni");
    assert_eq!(compose(&mut router, 1, "ni")["outcome"], "Passthrough");
    assert_eq!(
        key(&mut router, 1, ',' as u32, Some(','), false)["outcome"],
        "Passthrough"
    );
    // 再单击一次回中文
    key(&mut router, 1, 0xA1, None, false);
    key(&mut router, 1, 0xA1, None, true);
    assert_eq!(compose(&mut router, 1, "ni")["outcome"], "Consumed");
    // 纯英文下单击左 Shift 也回中文
    key(&mut router, 1, 13, None, false);
    key(&mut router, 1, 0xA1, None, false);
    key(&mut router, 1, 0xA1, None, true);
    key(&mut router, 1, 16, None, false);
    key(&mut router, 1, 16, None, true);
    assert_eq!(compose(&mut router, 1, "ni")["outcome"], "Consumed");
}

#[test]
fn right_shift_acts_like_shift_when_plain_english_is_off() {
    let mut router = router_with(RouterConfig {
        right_shift_english: false,
        ..Default::default()
    });
    // 右 Shift 切到普通英文模式：字母照样进英文补全，不是纯英文的整键放行
    key(&mut router, 1, 0xA1, None, false);
    key(&mut router, 1, 0xA1, None, true);
    assert_eq!(compose(&mut router, 1, "ni")["outcome"], "Consumed");
    key(&mut router, 1, 13, None, false);
    // 左 Shift 回中文
    key(&mut router, 1, 16, None, false);
    key(&mut router, 1, 16, None, true);
    assert_eq!(
        compose(&mut router, 1, "ni")["frame"]["candidates"]["items"][0]["text"],
        "你"
    );
}

#[test]
fn focus_switches_preserve_context_text_but_reject_old_candidate_actions() {
    let mut router = router();
    let first = compose(&mut router, 1, "ni");
    caps(&mut router, 2, true, false, false);
    compose(&mut router, 2, "nihao");
    assert_eq!(
        event(
            &mut router,
            1,
            json!({"Candidate": {"identity": first["identity"], "index": 0}})
        )["commit"],
        json!(null)
    );
    assert_eq!(key(&mut router, 1, 13, None, false)["commit"], "ni");
    assert_eq!(key(&mut router, 2, 13, None, false)["commit"], "nihao");
}
