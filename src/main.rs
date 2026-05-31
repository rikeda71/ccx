// CLI エントリポイント。
// 実装は docs/12 §16 参照。
fn main() -> anyhow::Result<()> {
    ccx::cli::run()
}
