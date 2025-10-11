use crate::cli::ConfigCommand;
use crate::config::Config;
use crate::error::Result;

pub async fn execute(command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Init => {
            Config::create_sample()?;
        }
        ConfigCommand::Path => {
            let config_path = Config::config_file_path()?;
            println!("Config file path: {}", config_path.display());

            if config_path.exists() {
                println!("Status: File exists");

                // Try to load and show if it's valid
                match Config::load() {
                    Ok(config) => {
                        println!("Valid: Yes");
                        if config.is_complete() {
                            println!("Complete: Yes");
                            let (start_url, region) = config.get_sso_config()?;
                            println!("\nSSO Configuration:");
                            println!("  Start URL: {}", start_url);
                            println!("  Region: {}", region);
                        } else {
                            println!("Complete: No (missing start_url or region)");
                        }
                    }
                    Err(e) => {
                        println!("Valid: No");
                        println!("Error: {}", e);
                    }
                }
            } else {
                println!("Status: File does not exist");
                println!("\nTo create a sample config file, run:");
                println!("  awsom config init");
            }
        }
    }

    Ok(())
}
