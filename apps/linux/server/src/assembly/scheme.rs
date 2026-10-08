//! 输入方案的装配：拼音侧（全拼 / 双拼 / 注音 / 关）与形码侧（五笔），两边都开是混输。
//!
//! 码表的找法与 Windows `dispatch::code` 一致：用户目录优先，随包 `assets/wubi/` 兜底。
//! 选了五笔却没有码表时只警告并按拼音跑——配置说五笔、引擎一个字都打不出更糟。

use std::path::{Path, PathBuf};

use qingjian_core::Engine;
use qingjian_dictionary::CodeTable;
use qingjian_platform::{GeneralConfig, Scheme};

/// 用户目录下的码表（用户自己换的那份）。
const USER_TABLE: &str = "wubi/wubi86.tsv";

/// 随包数据里的码表，与 emoji / levels 一样走 `assets/`。
const BUNDLED_TABLE: &str = "assets/wubi/wubi86.tsv";

/// 找码表：用户目录优先，否则随包数据；都没有为 `None`。
pub fn find_code_table(user_dir: Option<&Path>, bundled_root: &Path) -> Option<PathBuf> {
    user_dir
        .map(|dir| dir.join(USER_TABLE))
        .filter(|path| path.is_file())
        .or_else(|| {
            let bundled = bundled_root.join(BUNDLED_TABLE);
            bundled.is_file().then_some(bundled)
        })
}

/// 按 `[general] scheme` 与 `[general] wubi` 装配引擎；`table` 是 [`find_code_table`] 找到的码表。
pub fn apply_scheme(engine: &mut Engine, general: &GeneralConfig, table: Option<&Path>) {
    let pinyin = general.scheme();
    let wubi = general.wubi();
    engine.set_shuangpin(pinyin.shuangpin());
    engine.set_zhuyin_mode(pinyin == Scheme::Zhuyin);
    // 拼音侧关掉且五笔开着才是「只用形码」；两边都关着时留拼音兜底（否则一个候选都没有）
    engine.set_phonetic(pinyin.is_on() || !wubi);
    if !wubi {
        engine.set_code_table(None);
        return;
    }
    let Some(path) = table else {
        tracing::warn!(table = BUNDLED_TABLE, "选了五笔但找不到码表，仍按拼音输入");
        engine.set_code_table(None);
        return;
    };
    match CodeTable::from_path(path) {
        Ok(table) => {
            tracing::info!(table = %path.display(), entries = table.len(), "形码码表已载入");
            engine.set_code_table(Some(table));
        }
        Err(error) => {
            tracing::error!(%error, table = %path.display(), "形码码表读不了，仍按拼音输入");
            engine.set_code_table(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_dictionary::Dictionary;

    fn general(scheme: &str, wubi: &str) -> GeneralConfig {
        GeneralConfig {
            scheme: scheme.into(),
            wubi: wubi.into(),
            ..GeneralConfig::default()
        }
    }

    #[test]
    fn mixed_input_loads_the_table_and_shows_codes() {
        let dir =
            std::env::temp_dir().join(format!("qingjian-linux-scheme-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("wubi")).unwrap();
        std::fs::write(dir.join(USER_TABLE), "你好\twqvb\t100\n").unwrap();
        let table = find_code_table(Some(&dir), Path::new("/nonexistent"));
        assert_eq!(table.as_deref(), Some(dir.join(USER_TABLE).as_path()));

        let mut engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap());
        apply_scheme(&mut engine, &general("pinyin", "wubi86"), table.as_deref());
        assert!(engine.is_code_mode());
        engine.set_input("nihao");
        let items = engine.query().unwrap().candidates.items;
        assert_eq!(items[0].text, "你好");
        assert_eq!(items[0].reading.as_deref(), Some("wqvb"));

        apply_scheme(&mut engine, &general("pinyin", ""), table.as_deref());
        assert!(!engine.is_code_mode());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_table_falls_back_to_pinyin() {
        let mut engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap());
        apply_scheme(&mut engine, &general("none", "wubi86"), None);
        assert!(!engine.is_code_mode());
        engine.set_input("nihao");
        assert_eq!(engine.query().unwrap().candidates.items[0].text, "你好");
    }
}
