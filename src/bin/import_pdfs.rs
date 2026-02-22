use sat_stream::config::Config;
use sat_stream::db;
use sat_stream::pdf_extract;
use std::time::Instant;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    
    println!("📚 SAT-Stream Batch PDF Importer");
    println!("=================================");

    // 1. Get DB path
    let db_path = Config::db_path();
    println!("• Database: {:?}", db_path);

    // 2. Connect to DB
    let pool = db::init_db(db_path.to_str().unwrap()).await?;
    let initial_count = db::question_count(&pool).await?;
    println!("• Initial question count: {}", initial_count);

    // 3. Scan for PDFs
    let cwd = std::env::current_dir()?;
    println!("• Scanning directory: {:?}", cwd);
    
    // 4. Run extraction
    println!("\n🚀 Starting AI extraction (this may take a while)...");
    println!("   Using: pdftotext + llama.cpp (Qwen2.5-1.5B)");
    
    let start = Instant::now();
    match pdf_extract::extract_from_directory(&pool, cwd.to_str().unwrap()).await {
        Ok((count, summary)) => {
            let duration = start.elapsed();
            println!("\n✅ Batch Import Complete!");
            println!("• Time taken: {:.1?}", duration);
            println!("• Summary: {}", summary);
            println!("• Questions added: {}", count);
            println!("• Total questions in DB: {}", db::question_count(&pool).await?);
        }
        Err(e) => {
            println!("\n❌ Error during extraction:");
            println!("{}", e);
            if e.to_string().contains("llama-cli") {
                let checks = pdf_extract::check_readiness();
                println!("\nDiagnostics:");
                for (name, ok, msg) in checks {
                    println!("  {}: {}", name, msg);
                }
            }
        }
    }

    Ok(())
}
