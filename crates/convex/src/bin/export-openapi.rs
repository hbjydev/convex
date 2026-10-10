fn main() -> anyhow::Result<()> {
    println!("{}", convex_api::openapi().to_pretty_json()?);
    Ok(())
}
