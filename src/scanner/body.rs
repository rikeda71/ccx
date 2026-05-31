// 実装は docs/12 §9 参照
// skill/command/prompt 本文の変数・記法スキャナ。
// scan_body は検出のみ行い、本文を書き換えない（既定: read-only）。
// rewrite_body は --rewrite-body opt-in 時のみ呼ばれる。

use once_cell::sync::Lazy;
use regex::Regex;

use crate::core::transforms::ConvDir;

/// 検出パターンの種別（docs/09 の変数・記法と対応）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    /// 位置引数（0 基点）: $ARGUMENTS[0] / $ARGUMENTS[1] 等
    ArgIndexed,
    /// 名前付き引数: $name 形式
    ArgNamed,
    /// 環境変数参照: ${CLAUDE_*}
    EnvVar,
    /// 動的注入（inline）: !`cmd`
    DynamicInline,
    /// 動的注入（block）: ```! ... ```
    DynamicBlock,
    /// slash コマンド呼び出し: /skill-name
    InvokeSlash,
    /// 名前空間付き呼び出し: /namespace:skill
    InvokeNamespaced,
}

/// scan_body が本文中の検出事項に対して提案するアクション。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// 自動書き換え可能（rewrite_body で置換する）
    Rewrite,
    /// 警告のみ（自動書き換えしない。手動対応を促す）
    Warn,
    /// 出力から除去を提案（自動除去はしない）
    Drop,
}

/// 本文中の検出事項1件。
#[derive(Debug, Clone)]
pub struct BodyFinding {
    pub kind: FindingKind,
    /// マッチしたテキスト
    pub matched: String,
    /// 行番号（1 始まり）
    pub line: usize,
    /// 推奨アクション
    pub action: Action,
    /// action == Rewrite の場合の置換後テキスト
    pub rewrite: Option<String>,
    /// レポート用の説明メッセージ
    pub note: String,
}

// 静的に初期化される正規表現群（once_cell::Lazy で初期化）。
// 詳細パターンは §9 の検出パターン表を参照。

/// $ARGUMENTS[N] 形式（N は任意の整数）
static RE_ARG_INDEXED_BRACKET: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\$ARGUMENTS\[(\d+)\]").unwrap());

/// $N 形式（1 以上の整数）— Codex 側の位置引数
static RE_ARG_POSITIONAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$([1-9][0-9]*)").unwrap());

/// $ARGUMENTS（[N] が続く場合も続かない場合も含む。bare かどうかは後処理で判定）
static RE_ARG_BARE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$ARGUMENTS").unwrap());

/// $$ (x2c のみ)
static RE_DOLLAR_DOLLAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\$").unwrap());

/// $name 形式（小文字英字で始まる変数名）— $ARGUMENTS / ${...} 以外
static RE_ARG_NAMED: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$([a-z][a-z0-9_]*)").unwrap());

/// ${CLAUDE_*} 環境変数参照
static RE_ENV_VAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{CLAUDE_[A-Z_]+\}").unwrap());

/// 動的注入（inline）: !`cmd`
static RE_DYNAMIC_INLINE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(^|\s)!`[^`]+`").unwrap());

/// 動的注入（block）: ```!
static RE_DYNAMIC_BLOCK: Lazy<Regex> = Lazy::new(|| Regex::new(r"```!").unwrap());

/// 名前空間付き呼び出し: /namespace:skill（先に検出して InvokeSlash より優先）
static RE_INVOKE_NAMESPACED: Lazy<Regex> = Lazy::new(|| Regex::new(r"/[\w-]+:[\w-]+").unwrap());

/// slash コマンド呼び出し: /skill-name（名前空間なし）
static RE_INVOKE_SLASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"/[\w-]+").unwrap());

/// 本文をスキャンして検出事項の一覧を返す（本文は書き換えない）。
///
/// # 検出パターンと c2x の方針（§9 参照）
/// | 検出 | c2x の action |
/// |---|---|
/// | 位置引数（0 基点）$ARGUMENTS[0] | rewrite: index_shift +1（$ARGUMENTS[0]→$1） |
/// | bare $ARGUMENTS | warn（Codex は Custom Prompts のみ対応） |
/// | $$ (x2c のみ) | rewrite → $ |
/// | 名前付き引数 $name | warn（呼び出しが KEY=value に変わる） |
/// | ${CLAUDE_*} | drop（Codex に同等なし） |
/// | 動的注入 inline !`cmd` | warn（Codex 非対応） |
/// | 動的注入 block ```! | warn |
/// | /skill-name（slash 呼び出し） | warn → $name 提案 |
/// | /namespace:skill | drop |
///
/// NOTE: $0 は bash のスクリプト名と衝突するため auto-rewrite 対象外（propose のみ）。
///       $ARGUMENTS[0]→$1 の変換は [1] 以降のみ自動適用し、[0] は warn+propose。
pub fn scan_body(body: &str, dir: ConvDir) -> Vec<BodyFinding> {
    let mut findings = Vec::new();

    for (line_idx, line) in body.lines().enumerate() {
        let line_no = line_idx + 1;

        match dir {
            ConvDir::C2x => {
                scan_c2x_line(line, line_no, &mut findings);
            }
            ConvDir::X2c => {
                scan_x2c_line(line, line_no, &mut findings);
            }
        }
    }

    findings
}

fn scan_c2x_line(line: &str, line_no: usize, findings: &mut Vec<BodyFinding>) {
    // 1. 名前空間付き呼び出し（先に検出して InvokeSlash より優先）
    for cap in RE_INVOKE_NAMESPACED.find_iter(line) {
        findings.push(BodyFinding {
            kind: FindingKind::InvokeNamespaced,
            matched: cap.as_str().to_string(),
            line: line_no,
            action: Action::Drop,
            rewrite: None,
            note: "Codex に名前空間付き呼び出し概念なし。手動変換が必要です。".to_string(),
        });
    }

    // 2. ${CLAUDE_*} 環境変数参照
    for cap in RE_ENV_VAR.find_iter(line) {
        findings.push(BodyFinding {
            kind: FindingKind::EnvVar,
            matched: cap.as_str().to_string(),
            line: line_no,
            action: Action::Drop,
            rewrite: None,
            note: format!(
                "{} は Codex 側に等価な変数がないため除去が必要です。",
                cap.as_str()
            ),
        });
    }

    // 3. $ARGUMENTS[N] 位置引数（インデックスシフト）
    // まず $ARGUMENTS[N] を処理してから bare $ARGUMENTS を処理する
    // line に既に処理済みの位置を記録しておく
    let mut processed_positions: Vec<(usize, usize)> = Vec::new();

    for cap in RE_ARG_INDEXED_BRACKET.captures_iter(line) {
        let full_match = cap.get(0).unwrap();
        let idx_str = &cap[1];
        let idx: usize = idx_str.parse().unwrap_or(0);

        processed_positions.push((full_match.start(), full_match.end()));

        if idx == 0 {
            // $ARGUMENTS[0] → warn + propose $1 だが auto-rewrite しない（$0 は bash スクリプト名衝突）
            findings.push(BodyFinding {
                kind: FindingKind::ArgIndexed,
                matched: full_match.as_str().to_string(),
                line: line_no,
                action: Action::Warn,
                rewrite: Some("$1".to_string()),
                note: "$ARGUMENTS[0] → $1 への変換を提案。$0 は bash スクリプト名と衝突するため自動変換は行いません。手動確認が必要です。".to_string(),
            });
        } else {
            // $ARGUMENTS[N]（N>=1）→ $N に自動書き換え可能
            findings.push(BodyFinding {
                kind: FindingKind::ArgIndexed,
                matched: full_match.as_str().to_string(),
                line: line_no,
                action: Action::Rewrite,
                rewrite: Some(format!("${}", idx + 1)),
                note: format!("$ARGUMENTS[{}] → ${} (index +1)", idx, idx + 1),
            });
        }
    }

    // 4. bare $ARGUMENTS（[N] が続かない）
    // RE_ARG_BARE は $ARGUMENTS にマッチするが、その後に '[' が続く場合は
    // 既に RE_ARG_INDEXED_BRACKET で処理済みのため、ここでは bare のみを検出する
    for cap in RE_ARG_BARE.find_iter(line) {
        let start = cap.start();
        let end = cap.end();
        // $ARGUMENTS の直後が '[' なら bracket 形式 → スキップ
        let next_char = line.as_bytes().get(end).copied();
        if next_char == Some(b'[') {
            continue;
        }
        // 既処理位置と重複チェック
        if processed_positions
            .iter()
            .any(|(s, e)| start >= *s && start < *e)
        {
            continue;
        }
        findings.push(BodyFinding {
            kind: FindingKind::ArgIndexed,
            matched: cap.as_str().to_string(),
            line: line_no,
            action: Action::Warn,
            rewrite: None,
            note: "$ARGUMENTS は Codex の Custom Prompts のみ対応。Skill 本体では非対応です。手動対応が必要です。".to_string(),
        });
    }

    // 5. $name 形式（名前付き引数）— $ARGUMENTS と重複しないよう注意
    for cap in RE_ARG_NAMED.captures_iter(line) {
        let full_match = cap.get(0).unwrap();
        let name = &cap[1];
        // $ARGUMENTS の一部でないことを確認（RE_ARG_NAMED は小文字を対象とするが念のため）
        if name.starts_with("arguments") {
            continue;
        }
        // 既処理の位置と重複チェック
        let start = full_match.start();
        if processed_positions
            .iter()
            .any(|(s, e)| start >= *s && start < *e)
        {
            continue;
        }
        findings.push(BodyFinding {
            kind: FindingKind::ArgNamed,
            matched: full_match.as_str().to_string(),
            line: line_no,
            action: Action::Warn,
            rewrite: None,
            note: format!(
                "${} は Codex では KEY=value 形式の呼び出しに変わります。本文内の参照を確認してください。",
                name
            ),
        });
    }

    // 6. 動的注入 inline: !`cmd`
    for cap in RE_DYNAMIC_INLINE.find_iter(line) {
        findings.push(BodyFinding {
            kind: FindingKind::DynamicInline,
            matched: cap.as_str().trim().to_string(),
            line: line_no,
            action: Action::Warn,
            rewrite: None,
            note: "!`cmd` 動的注入は Codex 非対応。Codex ではリテラル文字列として扱われます。手動変換が必要です。".to_string(),
        });
    }

    // 7. 動的注入 block: ```!
    for cap in RE_DYNAMIC_BLOCK.find_iter(line) {
        findings.push(BodyFinding {
            kind: FindingKind::DynamicBlock,
            matched: cap.as_str().to_string(),
            line: line_no,
            action: Action::Warn,
            rewrite: None,
            note: "```! ブロック動的注入は Codex 非対応。手動変換が必要です。".to_string(),
        });
    }

    // 8. slash 呼び出し: /skill-name（名前空間なしのみ）
    // RE_INVOKE_NAMESPACED で既にヒットした位置を除外する
    let ns_positions: Vec<(usize, usize)> = RE_INVOKE_NAMESPACED
        .find_iter(line)
        .map(|m| (m.start(), m.end()))
        .collect();

    for cap in RE_INVOKE_SLASH.find_iter(line) {
        let start = cap.start();
        // 名前空間付きの一部でないことを確認
        if ns_positions.iter().any(|(s, e)| start >= *s && start < *e) {
            continue;
        }
        let skill_name = &cap.as_str()[1..]; // leading slash を除く
        findings.push(BodyFinding {
            kind: FindingKind::InvokeSlash,
            matched: cap.as_str().to_string(),
            line: line_no,
            action: Action::Warn,
            rewrite: Some(format!("${}", skill_name)),
            note: format!(
                "/{} は Codex では ${} として呼び出します。手動確認を推奨します。",
                skill_name, skill_name
            ),
        });
    }
}

fn scan_x2c_line(line: &str, line_no: usize, findings: &mut Vec<BodyFinding>) {
    // x2c 方向: Codex → Claude

    // 1. $$ → $ の rewrite
    for cap in RE_DOLLAR_DOLLAR.find_iter(line) {
        findings.push(BodyFinding {
            kind: FindingKind::ArgIndexed,
            matched: cap.as_str().to_string(),
            line: line_no,
            action: Action::Rewrite,
            rewrite: Some("$".to_string()),
            note: "$$ → $ に変換します。".to_string(),
        });
    }

    // 2. $N（1 以上）→ $ARGUMENTS[N-1] の rewrite
    for cap in RE_ARG_POSITIONAL.captures_iter(line) {
        let full_match = cap.get(0).unwrap();
        let idx_str = &cap[1];
        let idx: usize = idx_str.parse().unwrap_or(1);
        findings.push(BodyFinding {
            kind: FindingKind::ArgIndexed,
            matched: full_match.as_str().to_string(),
            line: line_no,
            action: Action::Rewrite,
            rewrite: Some(format!("$ARGUMENTS[{}]", idx - 1)),
            note: format!("${} → $ARGUMENTS[{}] (index -1)", idx, idx - 1),
        });
    }
}

/// scan_body が返した BodyFinding のうち action == Rewrite のもののみを本文に適用し、
/// 書き換え後の文字列を返す。
///
/// このメソッドは opts.rewrite_body == true の場合のみ handler の lower から呼ばれる。
/// 既定（rewrite_body == false）は本文をそのまま emit する。
pub fn rewrite_body(raw: &str, findings: &[BodyFinding]) -> String {
    // Rewrite のみを対象にする
    let rewrites: Vec<&BodyFinding> = findings
        .iter()
        .filter(|f| f.action == Action::Rewrite && f.rewrite.is_some())
        .collect();

    if rewrites.is_empty() {
        return raw.to_string();
    }

    // 行番号でグループ化して置換
    let lines: Vec<&str> = raw.lines().collect();
    let mut result_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();

    for finding in &rewrites {
        let line_idx = finding.line - 1; // 1始まり → 0始まり
        if line_idx < result_lines.len() {
            if let Some(rewrite) = &finding.rewrite {
                result_lines[line_idx] =
                    result_lines[line_idx].replacen(&finding.matched, rewrite, 1);
            }
        }
    }

    let mut output = result_lines.join("\n");
    // 元の末尾改行を保持
    if raw.ends_with('\n') {
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_body_arg_indexed_c2x() {
        let body = "Use $ARGUMENTS[0] and $ARGUMENTS[1] here";
        let findings = scan_body(body, ConvDir::C2x);
        let indexed: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == FindingKind::ArgIndexed)
            .collect();
        assert_eq!(indexed.len(), 2);
        // $ARGUMENTS[0] → warn (not auto-rewrite due to $0 conflict)
        let f0 = indexed
            .iter()
            .find(|f| f.matched == "$ARGUMENTS[0]")
            .unwrap();
        assert_eq!(f0.action, Action::Warn);
        // $ARGUMENTS[1] → rewrite to $2
        let f1 = indexed
            .iter()
            .find(|f| f.matched == "$ARGUMENTS[1]")
            .unwrap();
        assert_eq!(f1.action, Action::Rewrite);
        assert_eq!(f1.rewrite, Some("$2".to_string()));
    }

    #[test]
    fn test_scan_body_bare_arguments_c2x() {
        let body = "Pass $ARGUMENTS to the command";
        let findings = scan_body(body, ConvDir::C2x);
        let bare: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == FindingKind::ArgIndexed && f.matched == "$ARGUMENTS")
            .collect();
        assert_eq!(bare.len(), 1);
        assert_eq!(bare[0].action, Action::Warn);
    }

    #[test]
    fn test_scan_body_env_var_c2x() {
        let body = "Session: ${CLAUDE_SESSION_ID}";
        let findings = scan_body(body, ConvDir::C2x);
        let env: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == FindingKind::EnvVar)
            .collect();
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].action, Action::Drop);
    }

    #[test]
    fn test_scan_body_dynamic_inline_c2x() {
        let body = "Run !`git diff` to see changes";
        let findings = scan_body(body, ConvDir::C2x);
        let inline: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == FindingKind::DynamicInline)
            .collect();
        assert_eq!(inline.len(), 1);
        assert_eq!(inline[0].action, Action::Warn);
    }

    #[test]
    fn test_scan_body_namespaced_c2x() {
        let body = "Call /claude:deploy to deploy";
        let findings = scan_body(body, ConvDir::C2x);
        let ns: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == FindingKind::InvokeNamespaced)
            .collect();
        assert_eq!(ns.len(), 1);
        assert_eq!(ns[0].action, Action::Drop);
    }

    #[test]
    fn test_scan_body_slash_c2x() {
        let body = "Use /deploy command";
        let findings = scan_body(body, ConvDir::C2x);
        let slash: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == FindingKind::InvokeSlash)
            .collect();
        assert_eq!(slash.len(), 1);
        assert_eq!(slash[0].action, Action::Warn);
        assert_eq!(slash[0].rewrite, Some("$deploy".to_string()));
    }

    #[test]
    fn test_scan_body_dollar_dollar_x2c() {
        let body = "Escaped $$ dollar sign";
        let findings = scan_body(body, ConvDir::X2c);
        let dd: Vec<_> = findings.iter().filter(|f| f.matched == "$$").collect();
        assert_eq!(dd.len(), 1);
        assert_eq!(dd[0].action, Action::Rewrite);
        assert_eq!(dd[0].rewrite, Some("$".to_string()));
    }

    #[test]
    fn test_scan_body_positional_x2c() {
        let body = "Use $1 and $2 here";
        let findings = scan_body(body, ConvDir::X2c);
        let pos: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == FindingKind::ArgIndexed)
            .collect();
        assert_eq!(pos.len(), 2);
        let f1 = pos.iter().find(|f| f.matched == "$1").unwrap();
        assert_eq!(f1.rewrite, Some("$ARGUMENTS[0]".to_string()));
        let f2 = pos.iter().find(|f| f.matched == "$2").unwrap();
        assert_eq!(f2.rewrite, Some("$ARGUMENTS[1]".to_string()));
    }

    #[test]
    fn test_rewrite_body() {
        let body = "Use $ARGUMENTS[1] here\n";
        let findings = scan_body(body, ConvDir::C2x);
        let result = rewrite_body(body, &findings);
        assert!(result.contains("$2"), "Expected $2 in result: {}", result);
    }

    #[test]
    fn test_rewrite_body_no_rewrites() {
        let body = "No special patterns here\n";
        let findings = scan_body(body, ConvDir::C2x);
        let result = rewrite_body(body, &findings);
        assert_eq!(result, body);
    }

    #[test]
    fn test_scan_body_line_numbers() {
        let body = "line 1\nRun $ARGUMENTS[1]\nline 3\n";
        let findings = scan_body(body, ConvDir::C2x);
        let indexed: Vec<_> = findings
            .iter()
            .filter(|f| f.kind == FindingKind::ArgIndexed)
            .collect();
        assert!(!indexed.is_empty());
        assert_eq!(indexed[0].line, 2);
    }
}
