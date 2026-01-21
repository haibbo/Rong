use rong::*;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

fn main() {
    Rong::<RongJS>::builder()
        .build()
        .block_on(async |runtime, _receiver| {
            let ctx = runtime.context();

            ctx.global()
                .set(
                    "print",
                    JSFunc::new(&ctx, |msg: String| println!("{}", msg)),
                )
                .unwrap();
            rong_timer::init(&ctx)?;

            // Try to find the JS file in multiple locations to support different run locations
            let current_dir = std::env::current_dir().unwrap();
            let js_file_name = "timer_script.js";

            // Possible paths where the file might be located
            let possible_paths = [
                // When running from the project root
                current_dir
                    .join("modules/rong_timer/examples")
                    .join(js_file_name),
                // When running from modules/rong_timer directory
                current_dir.join("examples").join(js_file_name),
                // When running directly from the examples directory
                current_dir.join(js_file_name),
            ];

            // Find the first path that exists
            let js_path = possible_paths
                .iter()
                .find(|path| Path::new(path).exists())
                .ok_or_else(|| {
                    HostError::new(
                        rong::error::E_NOT_FOUND,
                        format!(
                            "Could not find timer_script.js in any of the expected locations.\n\
                         Tried:\n\
                         - {}\n\
                         - {}\n\
                         - {}\n\
                         Current directory: {}",
                            possible_paths[0].display(),
                            possible_paths[1].display(),
                            possible_paths[2].display(),
                            current_dir.display()
                        ),
                    )
                })?;

            println!("Found JS file at: {}", js_path.display());

            ctx.eval::<()>(Source::from_path(&ctx, js_path).await?)?;

            println!("Timers set up. Waiting for 5 seconds...");
            sleep(Duration::from_millis(5500)).await;

            println!("Program ending...");
            Ok(())
        })
        .unwrap();
}
