// ライブラリクレートのルート。
// integration tests (tests/*.rs) や外部クレートからのアクセスに使用する。
// main.rs は CLI エントリポイントとして別途存在する。
pub mod cli;
pub mod core;
pub mod degrade;
pub mod handlers;
pub mod scanner;
