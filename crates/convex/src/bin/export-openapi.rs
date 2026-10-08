fn main() -> anyhow::Result<()> {
    println!("{}", convex::http::openapi().to_pretty_json()?);
    Ok(())
}
