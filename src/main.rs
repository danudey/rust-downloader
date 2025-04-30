use std::{fs::File, process::exit};
use std::sync::Arc;
use std::io::copy;
use std::thread::{self, JoinHandle};

use reqwest::blocking;
use clap::Parser;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use url::Url;

use content_disposition::{parse_content_disposition, DispositionType};

#[derive(Parser)]
struct Cli {
    /// The URL to download from
    urls: Vec<String>,
}

fn download_file<'a>(urls: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {

    // Set our progress bar components
    let style = ProgressStyle::with_template("{prefix:.blue} {wide_bar:.blue/white} {percent}% • {bytes:.green}/{total_bytes:.green} • {binary_bytes_per_sec:.red} • {eta:.cyan}  ")
    .unwrap()
    .progress_chars("━╸━");

    let multiprog = Arc::new(MultiProgress::new());
    
    let mut handles: Vec<JoinHandle<_>> = vec![];

    for url in urls {
        // Parse our URL out so we can get a destination filename
        let parsed_url  = Url::parse(&url)?;
        let path_segments = parsed_url.path_segments().ok_or_else(|| "cannot be base")?;
        let url_filename = path_segments.last().ok_or_else(|| "I don't even know what's going on")?;

        // Make our HTTP request and get our response (headers)
        let response = blocking::get(url).map_err(|e| format!("Failed to send request: {}", e))?;

        // Bail out if some bad stuff happened
        if response.status().is_server_error() {
            println!("Got HTTP server error: {} {}", response.status().as_str(), response.status().canonical_reason().unwrap());
            exit(1);
        } else if  response.status().is_client_error() {
            println!("Got HTTP error: {} {}", response.status().as_str(), response.status().canonical_reason().unwrap());
            exit(2);
        }

        // Check the Content-Length header if we got one; otherwise, set it to zero
        let content_length = match response.content_length() {
            Some(length) => length,
            None => 0
        };

        let disposition = match response.headers().get("Content-Disposition") {
            Some(value) => value.to_str().unwrap(),
            None => ""
        };

        let disparsed = parse_content_disposition(disposition);
        let output_filename = if disparsed.disposition == DispositionType::Attachment {
            disparsed.filename_full().unwrap()
        } else {
            url_filename.to_string()
        };

        if output_filename.trim().is_empty() {
            println!("Cannot download URL: it doesn't have a file name so we don't know what to do");
            exit(3);
        }

        // let disposition = match response.headers().get("Content-Disposition") {
        //     Some(val) => val.to_str().unwrap(),
        //     None => ""
        // };


        // if !disposition.is_empty() {
        //     let parsed_disposition = parse_content_disposition(disposition);
        //     print!("{}", parsed_disposition.filename().unwrap());
        // }

        // Instantiate our progress bar; if we have a content length, we can use that
        // for the length of the input; otherwise, just use an indeterminate spinner
        let pb: ProgressBar = multiprog.add(ProgressBar::new(content_length).with_style(style.clone()));

        // Set the prefix to our filename so we can display it
        pb.set_prefix(String::from(url_filename));

        // Now we create our output file...
        let mut dest = File::create(url_filename).map_err(|e| format!("Failed to create file: {}", e))?;

        let handle = thread::spawn(move || {
            // ...and write the data to it as we get it
            let _ = copy(&mut pb.wrap_read(response), &mut dest).map_err(|e| format!("Failed to copy content: {}", e));
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

fn main() {
    let args= Cli::parse();

    let _ = download_file(args.urls).map_err(|e| println!("Application error: {}", e));


    // download_file(&args.url).map_err(|e| println!("Application error: {}", e))
}