use flv::data::FlvData;
use flv::error::FlvError;
use flv::parser_async::FlvDecoderStream;
use flv_fix::context::StreamerContext;
use flv_fix::pipeline::{FlvPipeline, PipelineConfig};
use flv_fix::writer_task::{FlvWriterTask, WriterError};
use futures::StreamExt;
use indicatif::HumanBytes;
use std::path::Path;
use tokio::fs::File;
use tokio::io::BufReader;
use tracing::info;

use crate::utils::format_bytes;
use crate::utils::progress::ProgressManager;

/// Process a single FLV file
pub async fn process_file(
    input_path: &Path,
    output_dir: &Path,
    config: PipelineConfig,
    enable_processing: bool,
    pb_manager: &mut ProgressManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = std::time::Instant::now();

    // Create output directory if it doesn't exist
    tokio::fs::create_dir_all(output_dir).await?;

    // Create base name for output files
    let base_name = input_path
        .file_stem()
        .ok_or("Invalid filename")?
        .to_string_lossy()
        .to_string();

    info!(
        path = %input_path.display(),
        processing_enabled = enable_processing,
        "Starting to process file"
    );

    // Open the file and create decoder stream
    let file = File::open(input_path).await?;
    let file_reader = BufReader::new(file);
    let file_size = file_reader.get_ref().metadata().await?.len();

    // Use provided progress manager or create a new one

    pb_manager.set_status(&format!("Processing {}", input_path.display()));
    // Create a file-specific progress bar
    let file_name = input_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    pb_manager.add_file_progress(&file_name);

    let mut decoder_stream = FlvDecoderStream::with_capacity(
        file_reader,
        1024 * 1024, // Input buffer capacity
    );

    // Create the input stream
    let (sender, receiver) = std::sync::mpsc::sync_channel::<Result<FlvData, FlvError>>(8);

    let mut process_task = None;

    let processed_stream = if enable_processing {
        // Processing mode: run through the processing pipeline
        info!(
            path = %input_path.display(),
            "Processing pipeline enabled, applying fixes and optimizations"
        );
        pb_manager.set_status("Processing with optimizations enabled");

        // Create streamer context and pipeline
        let context = StreamerContext::default();
        let pipeline = FlvPipeline::with_config(context, config);

        let (output_tx, output_rx) = std::sync::mpsc::sync_channel::<Result<FlvData, FlvError>>(8);

        process_task = Some(tokio::task::spawn_blocking(move || {
            let pipeline = pipeline.process();

            let input = std::iter::from_fn(|| {
                // Read from the receiver channel
                receiver.recv().map(Some).unwrap_or(None)
            });

            let mut output = |result: Result<FlvData, FlvError>| {
                // Send the processed result to the output channel
                output_tx.send(result).unwrap();
            };
            pipeline.process(input, &mut output).unwrap();
        }));
        output_rx
    } else {
        // Raw mode: bypass the pipeline entirely
        info!(
            path = %input_path.display(),
            "Processing pipeline disabled, outputting raw data"
        );
        pb_manager.set_status("Processing without optimizations");
        receiver
    };

    let output_dir = output_dir.to_path_buf();

    // Clone progress manager for the writer task
    let progress_clone = pb_manager.clone();

    // Create writer task and run it
    let writer_handle = tokio::task::spawn_blocking(move || {
        let mut writer_task = FlvWriterTask::new(output_dir, base_name)?;

        // Set up progress bar callbacks
        progress_clone.setup_writer_task_callbacks(&mut writer_task);

        writer_task.run(processed_stream)?;

        Ok::<_, WriterError>((
            writer_task.total_tags_written(),
            writer_task.files_created(),
        ))
    });

    // Wait for both tasks to complete
    // let bytes_processed = reader_handle.await?;
    let bytes_processed = 1000;

    while let Some(result) = decoder_stream.next().await {
        sender.send(result).unwrap()
    }
    drop(sender); // Close the channel to signal completion

    info!(
        path = %input_path.display(),
        "Finished processing input stream"
    );
    // reader_handle.await?;
    let (total_tags_written, files_created) = writer_handle.await??;

    if let Some(p) = process_task {
        p.await?;
    }

    let elapsed = start_time.elapsed();

    // Finish progress bars with summary
    pb_manager.finish(&format!(
        "Processed {} ({}) in {:?}",
        HumanBytes(bytes_processed),
        total_tags_written,
        elapsed
    ));

    info!(
        path = %input_path.display(),
        input_size = %format_bytes(file_size),
        duration = ?elapsed,
        processing_enabled = enable_processing,
        tags_processed = total_tags_written,
        files_created = files_created,
        "Processing complete"
    );

    Ok(())
}
